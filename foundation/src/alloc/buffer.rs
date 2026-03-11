use crate::alloc::{Allocated, Arena};
use crate::rust_alloc::rc::Rc;
use crate::rust_alloc::vec::Vec;

/// Incrementally builds a contiguous byte buffer inside an [`Arena`].
///
/// Appended bytes are staged into fixed-size internal pages. Calling
/// [`build`](Self::build) copies the written data into one final contiguous
/// allocation.
pub struct BufferBuilder {
    arena: Rc<Arena>,
    pages: Vec<Page>,
    page_size: usize,
}

struct Page {
    buffer: Allocated<u8>,
    cursor: usize,
}

/// Creates a new [`BufferBuilder`] that allocates internal pages from `arena`.
pub fn builder(arena: Rc<Arena>, page_size: usize) -> BufferBuilder {
    BufferBuilder {
        page_size,
        arena,
        pages: Vec::with_capacity(8),
    }
}

impl BufferBuilder {
    /// Returns the size used for newly allocated pages.
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// Appends bytes to the builder.
    ///
    /// Returns `None` if the arena cannot provide additional page storage.
    pub fn append(&mut self, bytes: impl AsRef<[u8]>) -> Option<()> {
        let bytes = bytes.as_ref();

        for page in &mut self.pages {
            if page.cursor + bytes.len() < page.buffer.len() {
                let slice = &mut page.buffer[page.cursor..page.cursor + bytes.len()];
                slice.copy_from_slice(bytes);
                page.cursor += bytes.len();

                return Some(());
            }
        }

        self.push_page(bytes)
    }

    /// Materializes all appended bytes into one contiguous arena allocation.
    pub fn build(&self) -> Option<Allocated<u8>> {
        let total_size = self.pages.iter().map(|p| p.cursor).sum();
        self.arena.allocate::<u8>(total_size).map(|mut buffer| {
            let mut cursor = 0;

            for page in &self.pages {
                let end = cursor + page.cursor;
                let slice = &mut buffer[cursor..end];
                slice.copy_from_slice(&page.buffer[..page.cursor]);
                cursor = end
            }

            buffer
        })
    }

    /// Clears the builder while retaining its internal pages for reuse.
    pub fn clear(&mut self) {
        for page in &mut self.pages {
            page.buffer[..].fill(0);
            page.cursor = 0;
        }
    }

    fn push_page(&mut self, bytes: &[u8]) -> Option<()> {
        let mut buffer = self.arena.allocate::<u8>(self.page_size)?;
        let size = bytes.len();

        buffer[..size].copy_from_slice(bytes);

        self.pages.push(Page {
            buffer,
            cursor: size,
        });

        Some(())
    }
}

#[cfg(feature = "std")]
impl std::io::Write for BufferBuilder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.append(buf).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "Unable to append bytes to buffer",
            )
        })?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
