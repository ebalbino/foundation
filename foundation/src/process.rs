//! Minimal child-process helpers.
//!
//! This module wraps `std::process` with a small API that integrates with the
//! arena-backed string builder types from [`crate::alloc`].
//!
//! Current behavior is intentionally narrow:
//!
//! - [`execute`] spawns a child process immediately
//! - standard input and output are piped
//! - standard error is not piped by default, so [`Process::stderr`] will usually
//!   return `None`
//! - each pipe can only be taken once because it consumes the child's handle
//!   from the underlying `std::process::Child`

use crate::alloc::{String, StringBuilder};
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};

/// A running child process.
pub struct Process {
    inner: Child,
}

/// Spawns a new child process.
///
/// The child pipes standard input and output, and leaves standard error
/// unpiped. Failure to spawn causes a panic.
pub fn execute<S: AsRef<OsStr>, I: IntoIterator<Item = S>>(cmd: S, args: I) -> Process {
    let inner = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    Process { inner }
}

impl Process {
    /// Writes bytes into the child's standard input and then closes the pipe.
    ///
    /// Returns `false` if standard input has already been taken.
    pub fn stdin<D: AsRef<[u8]>>(&mut self, input: D) -> bool {
        self.inner.stdin.take().is_some_and(|mut child| {
            child
                .write_all(input.as_ref())
                .expect("Unable to write to child process.");
            true
        })
    }

    /// Collects the child's standard output as raw bytes.
    ///
    /// Returns `None` if standard output has already been taken.
    pub fn stdout_bytes(&mut self) -> Option<Vec<u8>> {
        self.inner.stdout.take().map(read_pipe)
    }

    /// Collects the child's standard output into an arena-backed [`String`].
    ///
    /// The output is appended into `builder` in page-sized chunks and then built
    /// into a final string. Returns `None` if standard output has already been
    /// taken.
    pub fn stdout(&mut self, builder: &mut StringBuilder) -> Option<String> {
        self.stdout_bytes()
            .map(|buffer| build_string(builder, &buffer))
    }

    /// Collects the child's standard error as raw bytes.
    ///
    /// Returns `None` if stderr was not piped at spawn time or if it has already
    /// been taken.
    pub fn stderr_bytes(&mut self) -> Option<Vec<u8>> {
        self.inner.stderr.take().map(read_pipe)
    }

    /// Collects the child's standard error into an arena-backed [`String`].
    ///
    /// Returns `None` if stderr was not piped at spawn time or if it has already
    /// been taken.
    pub fn stderr(&mut self, builder: &mut StringBuilder) -> Option<String> {
        self.stderr_bytes()
            .map(|buffer| build_string(builder, &buffer))
    }
}

fn read_pipe(mut child: impl Read) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(4096);
    child
        .read_to_end(&mut buffer)
        .expect("Unable to read from child process.");
    buffer
}

fn build_string(builder: &mut StringBuilder, bytes: &[u8]) -> String {
    for chunk in bytes.chunks(builder.page_size()) {
        builder.append_bytes(chunk);
    }

    builder.build().expect("Unable to build string from output")
}
