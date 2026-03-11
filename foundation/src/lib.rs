#![no_std]

pub extern crate alloc as rust_alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod alloc;
pub mod encoding;
pub mod executor;
pub mod reflect;
pub mod serializer;
pub mod template;

#[cfg(feature = "std")]
pub mod file;
#[cfg(feature = "std")]
pub mod log;
#[cfg(feature = "std")]
pub mod process;
