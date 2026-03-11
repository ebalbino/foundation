use super::{Config, PushError, WorkQueue, config};
use std::io;
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

/// Creates a pool configuration with the requested worker count.
pub fn pool(workers: usize) -> PoolConfig {
    PoolConfig::new(workers)
}

/// Declarative thread-pool configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolConfig {
    workers: usize,
    thread: Config,
}

/// Errors returned by pool operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolError {
    /// The pool has already been closed.
    Closed,
}

/// A simple worker pool backed by a shared work queue.
pub struct Pool {
    queue: WorkQueue<Job>,
    workers: Vec<Worker>,
}

struct Worker {
    handle: JoinHandle<()>,
}

impl PoolConfig {
    /// Creates a new configuration for a pool with `workers` threads.
    pub fn new(workers: usize) -> Self {
        Self {
            workers,
            thread: config(),
        }
    }

    /// Sets the number of worker threads to spawn.
    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    /// Sets the base worker thread name.
    ///
    /// Worker indexes are appended as `name-0`, `name-1`, and so on.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.thread = self.thread.named(name);
        self
    }

    /// Sets the stack size for worker threads.
    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.thread = self.thread.stack_size(stack_size);
        self
    }

    /// Replaces the worker thread configuration directly.
    pub fn thread_config(mut self, thread: Config) -> Self {
        self.thread = thread;
        self
    }

    /// Builds and starts the worker pool.
    pub fn build(&self) -> io::Result<Pool> {
        if self.workers == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "worker pool requires at least one worker",
            ));
        }

        let queue = WorkQueue::new();
        let mut workers = Vec::with_capacity(self.workers);

        for index in 0..self.workers {
            let queue_for_thread = queue.clone();
            let config = self.thread.for_worker(index);
            let handle = config.spawn(move || worker_loop(queue_for_thread))?;
            workers.push(Worker { handle });
        }

        Ok(Pool { queue, workers })
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Pool {
    /// Queues a unit of work for execution by the next available worker.
    pub fn execute<F>(&self, job: F) -> Result<(), PoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.queue
            .push(Box::new(job))
            .map_err(|PushError| PoolError::Closed)
    }

    /// Prevents future submissions and lets workers drain outstanding work.
    pub fn close(&self) {
        self.queue.close();
    }

    /// Returns `true` once the pool has stopped accepting new work.
    pub fn is_closed(&self) -> bool {
        self.queue.is_closed()
    }

    /// Returns the number of worker threads managed by this pool.
    pub fn workers(&self) -> usize {
        self.workers.len()
    }

    /// Blocks until all workers exit after draining the queue.
    pub fn finish(mut self) -> thread::Result<()> {
        self.queue.close();

        for worker in std::mem::take(&mut self.workers) {
            worker.handle.join()?;
        }

        Ok(())
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.queue.close();
    }
}

fn worker_loop(queue: WorkQueue<Job>) {
    while let Some(job) = queue.pop() {
        job();
    }
}
