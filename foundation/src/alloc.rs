//! Arena-backed allocation primitives.
//!
//! This module provides a compact bump allocator (`Arena`) together with helpers
//! for building arena-backed byte buffers and UTF-8 strings.
//!
//! The core usage pattern is:
//!
//! - create an [`Arena`] with [`arena`]
//! - allocate typed slices with [`Arena::allocate`]
//! - optionally store non-`Copy` values with [`Allocated::pin`]
//! - reset the arena in bulk with [`Arena::reset`]
//!
//! [`Allocated<T>`] values weakly reference the source arena. If the arena is
//! dropped or reset to a new generation, those allocations become invalid and
//! dereference to empty slices instead of stale memory.

pub mod buffer;
pub mod string;

use core::fmt;
use std::cell::Cell;
use std::convert::{AsMut, AsRef};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::rc::{Rc, Weak};

pub use buffer::{BufferBuilder, builder as buffer_builder};
pub use string::builder::{StringBuilder, builder as string_builder};
pub use string::pool::{StringPool, pool as string_pool};
pub use string::{String, StringRef};

/// A bump-allocated byte arena.
///
/// The arena owns a contiguous byte buffer and advances a cursor for each
/// allocation. Storage is reclaimed in bulk by rewinding or resetting the arena.
pub struct Arena {
    buffer: Box<[u8]>,
    cursor: Cell<usize>,
    generation: Cell<usize>,
}

/// A typed slice allocated inside an [`Arena`].
///
/// This behaves like an owned handle to `&[T]` / `&mut [T]` backed by arena
/// storage. The handle remains cheap to clone, but it does not keep the arena
/// alive.
#[derive(Clone)]
pub struct Allocated<T> {
    arena: Weak<Arena>,
    offset: usize,
    count: usize,
    generation: usize,
    marker: PhantomData<T>,
}

/// A pinned, arena-owned value with a stable address.
///
/// Use this when a value cannot satisfy the `Copy` requirement used by
/// [`Arena::allocate`] but still needs to live inside the arena buffer.
pub struct Pinned<T> {
    arena: Rc<Arena>,
    offset: usize,
    marker: PhantomData<T>,
}

/// Creates a new arena with `capacity` bytes.
pub fn arena(capacity: usize) -> Rc<Arena> {
    let buffer = unsafe { Box::<[u8]>::new_zeroed_slice(capacity).assume_init() };

    Rc::new(Arena {
        buffer,
        cursor: Cell::new(0),
        generation: Cell::new(0),
    })
}

/// Copies an existing arena allocation into unused space in the same arena.
///
/// Returns `None` if the arena no longer exists or there is not enough capacity.
pub fn duplicate<T: Copy>(src: &Allocated<T>) -> Option<Allocated<T>> {
    let arena = src.arena.upgrade()?;
    arena.allocate::<T>(src.len()).map(|mut clone| {
        clone.copy_from_slice(src);
        clone
    })
}

impl Arena {
    fn allocate_raw<T>(self: &Rc<Self>, count: usize) -> Option<Allocated<T>> {
        let size = core::mem::size_of::<T>();
        let align = core::mem::align_of::<T>();
        let current_position = self.current_position();
        let aligned_position = (current_position + align - 1) & !(align - 1);
        let new_position = aligned_position + (size * count);
        let buffer = self.buffer();

        if new_position <= buffer.len() {
            self.seek(new_position);
            Some(Allocated {
                arena: Rc::downgrade(self),
                offset: aligned_position,
                count,
                generation: self.generation.get(),
                marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Allocates `count` values of `T` from the arena.
    ///
    /// The returned memory is aligned for `T`. Allocation fails with `None` when
    /// the arena does not have enough remaining capacity.
    pub fn allocate<T: Copy>(self: &Rc<Self>, count: usize) -> Option<Allocated<T>> {
        self.allocate_raw::<T>(count)
    }

    /// Returns the entire backing buffer for the arena.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer[..]
    }

    /// Returns the current cursor position in bytes.
    pub fn current_position(&self) -> usize {
        self.cursor.get()
    }

    /// Returns the arena generation used to invalidate old allocations.
    pub fn generation(&self) -> usize {
        self.generation.get()
    }

    /// Moves the arena cursor to an absolute byte position.
    pub fn seek(&self, position: usize) {
        self.cursor.set(position);
    }

    /// Rewinds the allocation cursor back to the start of the arena.
    ///
    /// Unlike [`reset`](Self::reset), this does not advance the generation.
    pub fn rewind(&self) {
        self.seek(0);
    }

    /// Clears the arena when it is uniquely owned.
    ///
    /// When the arena has a single strong reference, all bytes are zeroed, the
    /// cursor is rewound, and the generation is incremented so previously issued
    /// [`Allocated`] handles become invalid.
    pub fn reset(self: &mut Rc<Arena>) {
        if Rc::strong_count(self) == 1 {
            unsafe {
                std::ptr::write_bytes(self.buffer.as_ptr() as *mut u8, 0, self.buffer.len());
            }

            self.rewind();
            self.generation.set(self.generation.get() + 1);
        }
    }

    /// Places `value` in the arena and returns a pinned handle to it.
    ///
    /// This is the arena-backed alternative for values that must not move after
    /// construction.
    pub fn pin<T>(self: &Rc<Arena>, value: T) -> Option<Pin<Pinned<T>>> {
        let allocation = self.allocate_raw::<T>(1)?;
        let ptr = unsafe {
            allocation
                .arena_if_current()?
                .buffer
                .as_ptr()
                .add(allocation.offset) as *mut T
        };

        unsafe {
            ptr.write(value);
        }

        Some(unsafe {
            Pin::new_unchecked(Pinned {
                arena: self.clone(),
                offset: allocation.offset,
                marker: PhantomData,
            })
        })
    }

}

impl<T> Allocated<T> {
    fn arena_if_current(&self) -> Option<Rc<Arena>> {
        let arena = self.arena.upgrade()?;

        if arena.generation() == self.generation {
            Some(arena)
        } else {
            None
        }
    }
}

impl<T> Deref for Allocated<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self.arena_if_current() {
            Some(a) => unsafe {
                let ptr = a.buffer.as_ptr().add(self.offset) as *const T;
                std::slice::from_raw_parts(ptr, self.count)
            },
            None => &[],
        }
    }
}

impl<T> DerefMut for Allocated<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.arena_if_current() {
            Some(a) => unsafe {
                let ptr = a.buffer.as_ptr().add(self.offset) as *mut T;
                std::slice::from_raw_parts_mut(ptr, self.count)
            },
            None => &mut [],
        }
    }
}

impl<T> AsRef<[u8]> for Allocated<T> {
    fn as_ref(&self) -> &[u8] {
        match self.arena_if_current() {
            Some(a) => unsafe {
                let ptr = a.buffer.as_ptr().add(self.offset);
                std::slice::from_raw_parts(ptr, self.count * std::mem::size_of::<T>())
            },
            None => &[],
        }
    }
}

impl<T> AsMut<[u8]> for Allocated<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        match self.arena_if_current() {
            Some(a) => unsafe {
                let ptr = a.buffer.as_ptr().add(self.offset) as *mut u8;
                std::slice::from_raw_parts_mut(ptr, self.count * std::mem::size_of::<T>())
            },
            None => &mut [],
        }
    }
}

impl<T> Read for Allocated<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let inner = self.as_ref();

        if inner.len() == buf.len() {
            buf.copy_from_slice(inner);
            Ok(inner.len())
        } else {
            Err(io::Error::other("Buffer not same size"))
        }
    }
}

impl<T> Write for Allocated<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let inner = self.as_mut();

        if inner.len() == buf.len() {
            inner.copy_from_slice(buf);
            Ok(inner.len())
        } else {
            Err(io::Error::other("Buffer not same size"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Arena")
            .field("allocated", &self.cursor)
            .finish()
    }
}

impl<T: Debug> Debug for Allocated<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> PartialEq<[u8]> for Allocated<T> {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_ref() == other
    }
}

impl<T> Hash for Allocated<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl<T> Deref for Pinned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.arena.buffer.as_ptr().add(self.offset) as *const T) }
    }
}

impl<T> DerefMut for Pinned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.arena.buffer.as_ptr().add(self.offset) as *mut T) }
    }
}

impl<T> Pinned<T> {
    /// Projects a pinned `Pinned<T>` reference to a pinned `T` reference.
    pub fn as_pin_ref(self: Pin<&Self>) -> Pin<&T> {
        unsafe { self.map_unchecked(|value| &**value) }
    }

    /// Projects a pinned mutable `Pinned<T>` reference to a pinned mutable `T`.
    pub fn as_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|value| &mut **value) }
    }
}

impl<T> Drop for Pinned<T> {
    fn drop(&mut self) {
        unsafe {
            std::ptr::drop_in_place(self.deref_mut());
        }
    }
}
