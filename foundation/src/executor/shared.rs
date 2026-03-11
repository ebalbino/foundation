use super::ExecutorError;
use std::cell::RefCell;
use std::rc::Rc;

/// Shared access to the executor-owned value.
///
/// This is internally backed by `Rc<RefCell<T>>`, so reads and writes are local
/// to a single thread and enforce Rust borrowing rules at runtime.
pub struct Shared<T> {
    value: Rc<RefCell<T>>,
}

impl<T> Shared<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
        }
    }

    pub(crate) fn into_inner(self) -> Result<T, ExecutorError> {
        Rc::try_unwrap(self.value)
            .map(RefCell::into_inner)
            .map_err(|_| ExecutorError::OutstandingReferences)
    }

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

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}
