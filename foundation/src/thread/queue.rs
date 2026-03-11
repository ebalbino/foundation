use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

/// Creates a new shared work queue.
pub fn work_queue<T>() -> WorkQueue<T> {
    WorkQueue::new()
}

/// Error returned when pushing to a closed queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushError;

/// A blocking FIFO queue for work handoff between threads.
pub struct WorkQueue<T> {
    shared: Arc<Shared<T>>,
}

struct Shared<T> {
    state: Mutex<State<T>>,
    ready: Condvar,
}

struct State<T> {
    items: VecDeque<T>,
    closed: bool,
}

impl<T> WorkQueue<T> {
    /// Creates an empty queue.
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    items: VecDeque::new(),
                    closed: false,
                }),
                ready: Condvar::new(),
            }),
        }
    }

    /// Pushes an item onto the queue.
    ///
    /// Returns [`PushError`] when the queue has been closed.
    pub fn push(&self, item: T) -> Result<(), PushError> {
        let mut state = self.shared.state.lock().unwrap();

        if state.closed {
            return Err(PushError);
        }

        state.items.push_back(item);
        drop(state);
        self.shared.ready.notify_one();
        Ok(())
    }

    /// Attempts to pop an item without blocking.
    pub fn try_pop(&self) -> Option<T> {
        let mut state = self.shared.state.lock().unwrap();
        state.items.pop_front()
    }

    /// Pops the next available item, blocking until work arrives or the queue closes.
    pub fn pop(&self) -> Option<T> {
        let mut state = self.shared.state.lock().unwrap();

        loop {
            if let Some(item) = state.items.pop_front() {
                return Some(item);
            }

            if state.closed {
                return None;
            }

            state = self.shared.ready.wait(state).unwrap();
        }
    }

    /// Prevents any future pushes and wakes blocked consumers.
    pub fn close(&self) {
        let mut state = self.shared.state.lock().unwrap();
        state.closed = true;
        drop(state);
        self.shared.ready.notify_all();
    }

    /// Returns `true` once the queue has been closed.
    pub fn is_closed(&self) -> bool {
        let state = self.shared.state.lock().unwrap();
        state.closed
    }

    /// Returns the number of queued items.
    pub fn len(&self) -> usize {
        let state = self.shared.state.lock().unwrap();
        state.items.len()
    }

    /// Returns `true` when no items are currently queued.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Clone for WorkQueue<T> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<T> Default for WorkQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}
