//! Lightweight helpers for configuring threads declaratively.
//!
//! This module keeps close to `std::thread` while reducing the boilerplate
//! around repeatedly configuring names and stack sizes. It also includes a
//! small work queue and thread pool for simple multi-threaded pipelines.
//!
//! ```
//! use foundation::thread;
//!
//! let handle = thread::named("worker").spawn(|| 2 + 2).unwrap();
//! assert_eq!(handle.join().unwrap(), 4);
//! ```

mod config;
mod pool;
mod queue;

pub use config::{Config, config, named, scope, spawn};
pub use pool::{Pool, PoolConfig, PoolError, pool};
pub use queue::{PushError, WorkQueue, work_queue};
