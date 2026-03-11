use std::io;
use std::thread::{self, Builder, JoinHandle, Scope, ScopedJoinHandle};

/// Declarative thread configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    name: Option<String>,
    stack_size: Option<usize>,
}

/// Creates a default thread configuration.
pub fn config() -> Config {
    Config::default()
}

/// Creates a thread configuration with a thread name already set.
pub fn named(name: impl Into<String>) -> Config {
    Config::default().named(name)
}

/// Spawns a thread with the default configuration.
pub fn spawn<F, T>(f: F) -> io::Result<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    config().spawn(f)
}

/// Runs a scoped threading region.
pub fn scope<'env, F, R>(f: F) -> R
where
    F: for<'scope> FnOnce(&Scope<'scope, 'env>) -> R,
{
    thread::scope(f)
}

impl Config {
    /// Sets the thread name.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the thread stack size in bytes.
    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    /// Converts this configuration into a [`std::thread::Builder`].
    pub fn builder(&self) -> Builder {
        let mut builder = Builder::new();

        if let Some(name) = &self.name {
            builder = builder.name(name.clone());
        }

        if let Some(stack_size) = self.stack_size {
            builder = builder.stack_size(stack_size);
        }

        builder
    }

    /// Spawns an owned thread using this configuration.
    pub fn spawn<F, T>(&self, f: F) -> io::Result<JoinHandle<T>>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.builder().spawn(f)
    }

    /// Spawns a scoped thread using this configuration.
    pub fn spawn_scoped<'scope, 'env, F, T>(
        &self,
        scope: &'scope Scope<'scope, 'env>,
        f: F,
    ) -> io::Result<ScopedJoinHandle<'scope, T>>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        self.builder().spawn_scoped(scope, f)
    }

    pub(crate) fn for_worker(&self, index: usize) -> Self {
        let mut config = self.clone();

        if let Some(name) = &self.name {
            config.name = Some(format!("{name}-{index}"));
        }

        config
    }
}
