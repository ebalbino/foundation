//! A small single-threaded cooperative executor.
//!
//! Tasks are stored inside an arena and polled in FIFO order. Progress is
//! cooperative: a task must either complete or arrange to wake itself before
//! returning `Poll::Pending`. Helpers like [`yield_now`] do this explicitly.
//!
//! The executor is designed for deterministic stepping:
//!
//! - [`Executor::step`] polls at most one task
//! - [`Executor::run`] keeps stepping until completion or stall
//! - [`Executor::resolve`] runs the executor to completion and returns the
//!   owned shared state when no external [`Shared`] handles remain

use crate::alloc::{Allocated, Arena};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Wake, Waker};

/// A cooperative executor that owns shared state of type `T`.
///
/// Spawned tasks receive a [`Shared<T>`] handle, allowing them to read or mutate
/// the shared value while the executor remains single-threaded.
pub struct Executor<T> {
    arena: Rc<Arena>,
    shared: Shared<T>,
    tasks: VecDeque<Task>,
}

struct Task {
    inner: Pin<crate::alloc::Pinned<Pin<Box<dyn StdFuture<Output = ()> + 'static>>>>,
}

/// Shared access to the executor-owned value.
///
/// This is internally backed by `Rc<RefCell<T>>`, so reads and writes are local
/// to a single thread and enforce Rust borrowing rules at runtime.
pub struct Shared<T> {
    value: Rc<RefCell<T>>,
}

/// Errors returned by executor driving operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    /// A task returned `Poll::Pending` without waking the executor's waker.
    ///
    /// In practice this means no further progress can be made unless some
    /// external actor polls the executor again under different conditions.
    Stalled,
    /// [`Executor::resolve`] could not recover ownership of the shared value
    /// because other [`Shared`] handles still exist.
    OutstandingReferences,
}

/// The result of a single executor step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// No tasks were available to poll.
    Idle,
    /// A task was polled and either completed or re-queued after waking itself.
    Progressed,
}

/// Creates a new [`Executor`] backed by `arena` and initialized with `value`.
pub fn executor<T>(arena: Rc<Arena>, value: T) -> Executor<T> {
    Executor {
        arena,
        shared: Shared {
            value: Rc::new(RefCell::new(value)),
        },
        tasks: VecDeque::new(),
    }
}

impl<T> Executor<T> {
    /// Returns a cloneable handle to the shared executor state.
    pub fn shared(&self) -> Shared<T> {
        self.shared.clone()
    }

    /// Spawns a new task into the executor.
    ///
    /// The task is created from `f`, which receives a [`Shared<T>`] handle to the
    /// executor state. Returns `None` if the arena cannot store the task future.
    pub fn spawn<F, Fut>(&mut self, f: F) -> Option<()>
    where
        F: FnOnce(Shared<T>) -> Fut + 'static,
        Fut: StdFuture<Output = ()> + 'static,
    {
        let boxed = Box::new(f(self.shared())) as Box<dyn StdFuture<Output = ()> + 'static>;
        let pinned = unsafe { Pin::new_unchecked(boxed) };
        let inner = Allocated::pin(&self.arena, pinned)?;

        self.tasks.push_back(Task { inner });

        Some(())
    }

    /// Returns the number of tasks that are still queued.
    pub fn pending(&self) -> usize {
        self.tasks.len()
    }

    /// Returns `true` when no queued tasks remain.
    pub fn is_complete(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Polls at most one queued task.
    ///
    /// A completed task is removed. A pending task is re-queued. If a task
    /// returns `Poll::Pending` without waking the executor, this returns
    /// [`ExecutorError::Stalled`].
    pub fn step(&mut self) -> Result<Step, ExecutorError> {
        let Some(mut task) = self.tasks.pop_front() else {
            return Ok(Step::Idle);
        };

        let waker_state = Arc::new(WakerState::default());
        let waker = Waker::from(waker_state.clone());
        let mut context = Context::from_waker(&waker);

        waker_state.reset();

        match task.inner.as_mut().get_mut().as_mut().poll(&mut context) {
            Poll::Ready(()) => Ok(Step::Progressed),
            Poll::Pending => {
                if waker_state.was_woken() {
                    self.tasks.push_back(task);
                    Ok(Step::Progressed)
                } else {
                    self.tasks.push_back(task);
                    Err(ExecutorError::Stalled)
                }
            }
        }
    }

    /// Repeatedly steps the executor until all tasks finish or execution stalls.
    pub fn run(&mut self) -> Result<(), ExecutorError> {
        loop {
            match self.step()? {
                Step::Idle => return Ok(()),
                Step::Progressed => continue,
            }
        }
    }

    /// Runs all tasks and returns the final shared value.
    ///
    /// This fails with [`ExecutorError::OutstandingReferences`] if any cloned
    /// [`Shared<T>`] handles still exist after task completion.
    pub fn resolve(mut self) -> Result<T, ExecutorError> {
        self.run()?;
        drop(self.tasks);

        Rc::try_unwrap(self.shared.value)
            .map(RefCell::into_inner)
            .map_err(|_| ExecutorError::OutstandingReferences)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T> Shared<T> {
    /// Mutably borrows the shared value for the duration of `f`.
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut value = self.value.borrow_mut();
        f(&mut value)
    }

    /// Immutably borrows the shared value for the duration of `f`.
    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let value = self.value.borrow();
        f(&value)
    }
}

#[derive(Default)]
struct WakerState {
    woken: AtomicBool,
}

impl WakerState {
    fn reset(&self) {
        self.woken.store(false, Ordering::SeqCst);
    }

    fn was_woken(&self) -> bool {
        self.woken.load(Ordering::SeqCst)
    }
}

impl Wake for WakerState {
    fn wake(self: Arc<Self>) {
        self.woken.store(true, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.woken.store(true, Ordering::SeqCst);
    }
}

/// Returns a future that yields once before completing.
///
/// This is useful for cooperative scheduling when a task wants to let other
/// queued tasks run before continuing.
pub fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

/// Future returned by [`yield_now`].
pub struct YieldNow {
    yielded: bool,
}

impl StdFuture for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
