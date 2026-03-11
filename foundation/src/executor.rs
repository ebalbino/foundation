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

mod shared;
mod task;
mod wake;
mod yield_now;

use crate::alloc::Arena;
use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub use shared::Shared;
use task::Task;
use wake::waker;
pub use yield_now::{YieldNow, yield_now};

/// A cooperative executor that owns shared state of type `T`.
///
/// Spawned tasks receive a [`Shared<T>`] handle, allowing them to read or mutate
/// the shared value while the executor remains single-threaded.
pub struct Executor<T> {
    arena: Rc<Arena>,
    shared: Shared<T>,
    tasks: VecDeque<Pin<crate::alloc::Pinned<Task>>>,
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
        shared: Shared::new(value),
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
        let future = self.arena.pin(f(self.shared()))?;
        let task = self.arena.pin(Task::new(future))?;
        self.tasks.push_back(task);
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

        task.waker_state.reset();
        let waker = waker(&task.waker_state);
        let mut context = Context::from_waker(&waker);

        match unsafe { task.as_mut().poll(&mut context) } {
            Poll::Ready(()) => Ok(Step::Progressed),
            Poll::Pending => {
                if task.waker_state.was_woken() {
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
        self.shared.into_inner()
    }
}
