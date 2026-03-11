use crate::alloc::Arena;
use crate::alloc::buffer::{self, BufferBuilder};
use crate::alloc::string::String;
use crate::rust_alloc::rc::Rc;
use core::fmt::Write;

/// Incrementally builds arena-backed UTF-8 text.
pub struct StringBuilder {
    buffer: BufferBuilder,
}

/// Creates a new [`StringBuilder`] with the provided page size.
pub fn builder(arena: Rc<Arena>, page_size: usize) -> StringBuilder {
    StringBuilder {
        buffer: buffer::builder(arena, page_size),
    }
}

impl StringBuilder {
    /// Returns the page size of the underlying byte builder.
    pub fn page_size(&self) -> usize {
        self.buffer.page_size()
    }

    /// Appends UTF-8 text to the builder.
    pub fn append(&mut self, s: impl AsRef<str>) -> Option<()> {
        self.buffer.append(s.as_ref())
    }

    /// Appends raw bytes to the builder.
    ///
    /// The caller is responsible for keeping the final contents valid UTF-8.
    pub fn append_bytes(&mut self, b: impl AsRef<[u8]>) -> Option<()> {
        self.buffer.append(b.as_ref())
    }

    /// Builds the accumulated contents into an arena-backed [`String`].
    pub fn build(&self) -> Option<String> {
        self.buffer.build().map(String::from)
    }

    /// Clears the accumulated contents while retaining internal pages.
    pub fn clear(&mut self) {
        self.buffer.clear()
    }
}

impl Write for StringBuilder {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        self.append(s).ok_or(core::fmt::Error)
    }
}
