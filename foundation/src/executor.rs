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

use crate::alloc::Arena;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// A cooperative executor that owns shared state of type `T`.
///
/// Spawned tasks receive a [`Shared<T>`] handle, allowing them to read or mutate
/// the shared value while the executor remains single-threaded.
pub struct Executor<T> {
    arena: Rc<Arena>,
    shared: Shared<T>,
    tasks: VecDeque<Task>,
    queue_capacity: usize,
}

struct Task {
    arena: Rc<Arena>,
    offset: usize,
    poll: unsafe fn(&Rc<Arena>, usize, &mut Context<'_>) -> Poll<()>,
    drop: unsafe fn(Rc<Arena>, usize),
    waker_state: Pin<crate::alloc::Pinned<WakerState>>,
}

/// Shared access to the executor-owned value.
///
/// This is internally backed by `Rc<RefCell<T>>`, so reads and writes are local
/// to a single thread and enforce Rust borrowing rules at runtime.
pub struct Shared<T> {
    value: Rc<RefCell<T>>,
}

/// A local-only analogue to [`std::task::Wake`].
pub trait LocalWake {
    /// Wake this task.
    fn wake(&self);

    /// Wake this task without consuming the local waker.
    fn wake_by_ref(&self) {
        self.wake();
    }
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
        queue_capacity: usize::MAX,
    }
}

/// Creates a new [`Executor`] backed by `arena` and initialized with `value`.
///
/// At most `queue_capacity` spawned tasks may be queued at once.
pub fn executor_with_capacity<T>(arena: Rc<Arena>, value: T, queue_capacity: usize) -> Executor<T> {
    Executor {
        arena,
        shared: Shared {
            value: Rc::new(RefCell::new(value)),
        },
        tasks: VecDeque::with_capacity(queue_capacity),
        queue_capacity,
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
        if self.tasks.len() >= self.queue_capacity {
            return None;
        }

        let inner = self.arena.pin(f(self.shared()))?;
        self.tasks.push_back(Task::new(inner));

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

        let mut context = Context::from_waker(Waker::noop());
        let local_waker = task.local_waker().clone();

        task.reset_waker();

        match with_local_waker(&local_waker, || task.poll(&mut context)) {
            Poll::Ready(()) => Ok(Step::Progressed),
            Poll::Pending => {
                if task.was_woken() {
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

impl Task {
    fn new<Fut>(inner: Pin<crate::alloc::Pinned<Fut>>) -> Self
    where
        Fut: StdFuture<Output = ()> + 'static,
    {
        let (arena, offset) = crate::alloc::Pinned::into_raw_parts(inner);
        let waker_state = arena.pin(WakerState::default()).expect(
            "task future allocated successfully but executor could not allocate task waker state",
        );

        Self {
            arena,
            offset,
            poll: poll_future::<Fut>,
            drop: drop_future::<Fut>,
            waker_state,
        }
    }

    fn poll(&mut self, context: &mut Context<'_>) -> Poll<()> {
        unsafe { (self.poll)(&self.arena, self.offset, context) }
    }

    fn reset_waker(&self) {
        self.waker_state.reset();
    }

    fn was_woken(&self) -> bool {
        self.waker_state.was_woken()
    }

    fn local_waker(&self) -> LocalWaker {
        LocalWaker::from_borrowed(self.waker_state.as_ref().get_ref())
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.arena.clone(), self.offset);
        }
    }
}

unsafe fn poll_future<Fut>(arena: &Rc<Arena>, offset: usize, context: &mut Context<'_>) -> Poll<()>
where
    Fut: StdFuture<Output = ()> + 'static,
{
    let future = unsafe { crate::alloc::Pinned::<Fut>::pin_from_raw_parts(arena, offset) };
    StdFuture::poll(future, context)
}

unsafe fn drop_future<Fut>(arena: Rc<Arena>, offset: usize)
where
    Fut: 'static,
{
    drop(unsafe { crate::alloc::Pinned::<Fut>::from_raw_parts(arena, offset) });
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
    woken: Cell<bool>,
}

impl WakerState {
    fn reset(&self) {
        self.woken.set(false);
    }

    fn was_woken(&self) -> bool {
        self.woken.get()
    }
}

impl LocalWake for WakerState {
    fn wake(&self) {
        self.woken.set(true);
    }

    fn wake_by_ref(&self) {
        self.woken.set(true);
    }
}

/// A local-only analogue to [`std::task::LocalWaker`].
pub struct LocalWaker {
    data: *const (),
    vtable: LocalWakerVTable,
    marker: PhantomData<Rc<()>>,
}

impl LocalWaker {
    /// Wakes the associated task, consuming this local waker.
    pub fn wake(self) {
        unsafe { (self.vtable.wake)(self.data) };
    }

    /// Wakes the associated task without consuming this local waker.
    pub fn wake_by_ref(&self) {
        unsafe { (self.vtable.wake_by_ref)(self.data) }
    }
}

impl Clone for LocalWaker {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            vtable: self.vtable,
            marker: PhantomData,
        }
    }
}

impl LocalWaker {
    fn from_borrowed<W: LocalWake + 'static>(waker: &W) -> Self {
        Self {
            data: waker as *const W as *const (),
            vtable: local_waker_vtable::<W>(),
            marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
struct LocalWakerVTable {
    wake: unsafe fn(*const ()),
    wake_by_ref: unsafe fn(*const ()),
}

fn local_waker_vtable<W: LocalWake + 'static>() -> LocalWakerVTable {
    LocalWakerVTable {
        wake: wake::<W>,
        wake_by_ref: wake_by_ref::<W>,
    }
}

unsafe fn wake<W: LocalWake + 'static>(waker: *const ()) {
    let waker = unsafe { &*(waker as *const W) };
    W::wake(waker);
}

unsafe fn wake_by_ref<W: LocalWake + 'static>(waker: *const ()) {
    let waker = unsafe { &*(waker as *const W) };
    W::wake_by_ref(waker);
}

thread_local! {
    static CURRENT_LOCAL_WAKER: RefCell<Option<LocalWaker>> = const { RefCell::new(None) };
}

fn with_local_waker<R>(local_waker: &LocalWaker, f: impl FnOnce() -> R) -> R {
    CURRENT_LOCAL_WAKER.with(|slot| {
        let previous = slot.replace(Some(local_waker.clone()));
        let result = f();
        slot.replace(previous);
        result
    })
}

fn current_local_waker() -> Option<LocalWaker> {
    CURRENT_LOCAL_WAKER.with(|slot| slot.borrow().clone())
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
            if let Some(local_waker) = current_local_waker() {
                local_waker.wake_by_ref();
            } else {
                cx.waker().wake_by_ref();
            }
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;

    struct CountingLocalWake {
        wakes: Cell<usize>,
    }

    impl CountingLocalWake {
        fn new() -> Self {
            Self {
                wakes: Cell::new(0),
            }
        }

        fn count(&self) -> usize {
            self.wakes.get()
        }
    }

    impl LocalWake for CountingLocalWake {
        fn wake(&self) {
            self.wakes.set(self.wakes.get() + 1);
        }
    }

    struct CountingWake {
        wakes: AtomicUsize,
    }

    impl CountingWake {
        fn new() -> Self {
            Self {
                wakes: AtomicUsize::new(0),
            }
        }

        fn count(&self) -> usize {
            self.wakes.load(Ordering::SeqCst)
        }
    }

    impl Wake for CountingWake {
        fn wake(self: Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn local_wake_by_ref_defaults_to_wake() {
        let wake = CountingLocalWake::new();

        <CountingLocalWake as LocalWake>::wake_by_ref(&wake);

        assert_eq!(wake.count(), 1);
    }

    #[test]
    fn shared_read_borrows_the_current_value() {
        let shared = Shared {
            value: Rc::new(RefCell::new(41_u32)),
        };

        let value = shared.read(|value| *value + 1);

        assert_eq!(value, 42);
    }

    #[test]
    fn waker_state_and_local_waker_wake_paths_set_the_flag() {
        let state = WakerState::default();
        state.wake();
        assert!(state.was_woken());

        state.reset();
        let local_waker = LocalWaker::from_borrowed(&state);
        local_waker.wake();
        assert!(state.was_woken());

        state.reset();
        let vtable = local_waker_vtable::<WakerState>();
        unsafe {
            (vtable.wake)(std::ptr::from_ref(&state).cast());
        }
        assert!(state.was_woken());
    }

    #[test]
    fn with_local_waker_restores_the_previous_waker_and_returns_the_result() {
        let outer = CountingLocalWake::new();
        let inner = CountingLocalWake::new();
        let outer_waker = LocalWaker::from_borrowed(&outer);
        let inner_waker = LocalWaker::from_borrowed(&inner);

        let result = with_local_waker(&outer_waker, || {
            current_local_waker().unwrap().wake_by_ref();

            with_local_waker(&inner_waker, || {
                current_local_waker().unwrap().wake();
            });

            current_local_waker().unwrap().wake_by_ref();
            7_u32
        });

        assert_eq!(result, 7);
        assert_eq!(outer.count(), 2);
        assert_eq!(inner.count(), 1);
        assert!(current_local_waker().is_none());
    }

    #[test]
    fn yield_now_uses_the_context_waker_without_a_local_waker() {
        let counter = Arc::new(CountingWake::new());
        let waker = Waker::from(counter.clone());
        let mut context = Context::from_waker(&waker);
        let mut future = std::pin::pin!(yield_now());

        assert_eq!(future.as_mut().poll(&mut context), Poll::Pending);
        assert_eq!(counter.count(), 1);
        assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(()));
    }
}
