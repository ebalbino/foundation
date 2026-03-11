pub mod builder;
pub mod pool;

use crate::alloc::{self, Allocated, Arena};
use crate::rust_alloc::rc::Rc;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};
use core::ops::Deref;

/// Immutable UTF-8 text stored in arena-backed memory.
#[derive(Clone)]
pub struct String {
    buffer: Allocated<u8>,
}

/// Borrowed view into UTF-8 text without lifetime tracking.
///
/// `StringRef` is useful as a cheap handle when the caller can guarantee the
/// underlying bytes stay alive.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringRef {
    buffer: *const u8,
    len: usize,
}

/// Allocates UTF-8 bytes into the arena and returns them as [`String`].
pub fn make(arena: Rc<Arena>, s: impl AsRef<[u8]>) -> Option<String> {
    let bytes = s.as_ref();
    arena.allocate::<u8>(bytes.len()).map(|mut buffer: Allocated<u8>| {
        buffer.copy_from_slice(bytes);

        String { buffer }
    })
}

/// Wraps an existing byte allocation as [`String`].
///
/// The caller must ensure the bytes contain valid UTF-8.
pub fn wrap(buffer: Allocated<u8>) -> String {
    String::from(buffer)
}

/// Duplicates a string into unused space in the same arena.
pub fn duplicate(s: &String) -> Option<String> {
    alloc::duplicate(&s.buffer).map(|buffer| String { buffer })
}

impl String {
    /// Returns a cheap borrowed view of the same text.
    pub fn borrow(&self) -> StringRef {
        StringRef {
            buffer: self.as_ptr(),
            len: self.len(),
        }
    }
}

impl Deref for String {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { core::str::from_utf8_unchecked(&self.buffer) }
    }
}

impl Deref for StringRef {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(self.buffer, self.len)) }
    }
}

impl AsRef<[u8]> for String {
    fn as_ref(&self) -> &[u8] {
        &self.buffer
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        self.deref()
    }
}

impl AsRef<[u8]> for StringRef {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<str> for StringRef {
    fn as_ref(&self) -> &str {
        &self
    }
}

impl PartialEq<str> for String {
    fn eq(&self, other: &str) -> bool {
        self.deref() == other
    }
}

impl PartialEq<[u8]> for String {
    fn eq(&self, other: &[u8]) -> bool {
        &self.buffer[..] == other
    }
}

impl PartialEq<String> for String {
    fn eq(&self, other: &String) -> bool {
        self.deref() == other.deref()
    }
}

impl PartialEq<str> for StringRef {
    fn eq(&self, other: &str) -> bool {
        self.deref() == other
    }
}

impl PartialEq<[u8]> for StringRef {
    fn eq(&self, other: &[u8]) -> bool {
        self[..].as_bytes() == other
    }
}

impl PartialEq<String> for StringRef {
    fn eq(&self, other: &String) -> bool {
        self.deref() == other.deref()
    }
}

impl Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Debug for StringRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Display for StringRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Hash for String {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.buffer.hash(state);
    }
}

impl Hash for StringRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self[..].hash(state);
    }
}

impl From<Allocated<u8>> for String {
    fn from(buffer: Allocated<u8>) -> Self {
        Self { buffer }
    }
}

impl From<&str> for StringRef {
    fn from(buffer: &str) -> Self {
        Self {
            buffer: buffer.as_ptr(),
            len: buffer.as_bytes().len(),
        }
    }
}
