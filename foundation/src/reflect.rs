//! Lightweight runtime type descriptions.
//!
//! The reflection model in this crate is intentionally structural. Types expose a
//! [`Description`] through [`Introspectable`], and those descriptions can be stored
//! in a [`TypeRegistry`] for later lookup by name.
//!
//! The metadata currently captures:
//!
//! - scalar base types through [`Base`]
//! - struct-like composites through [`Field`] and [`Value::Composite`]
//! - enum-like alternatives through [`Value::Enumeration`]
//! - pointer or handle-like wrappers through [`Value::Reference`]
//!
//! This module does not yet provide dynamic field traversal helpers over raw data.
//! [`Instance`] is the start of that story: it associates bytes with a registered
//! description, but today it is primarily a typed pairing of data and metadata.

use crate::alloc::Allocated;
use std::rc::Rc;

mod introspectable;
pub mod registry;
pub use introspectable::Introspectable;
pub use registry::TypeRegistry;

/// Primitive scalar kinds understood by the reflection system.
#[derive(Debug, Copy, Clone)]
pub enum Base {
    Void,
    Unsigned8,
    Unsigned16,
    Unsigned32,
    Unsigned64,
    UnsignedPtr,
    Signed8,
    Signed16,
    Signed32,
    Signed64,
    SignedPtr,
    Float32,
    Float64,
}

/// The high-level shape of a reflected type.
#[derive(Debug, Clone)]
pub enum Value {
    None,
    Scalar { base: Base },
    Composite { fields: Vec<Field> },
    Enumeration { values: Vec<Description> },
    Reference { pointee: Box<Description> },
}

/// A named field within a composite type description.
#[derive(Debug, Clone)]
pub struct Field {
    /// The reflected description of the field type.
    pub desc: Description,
    /// The source-level field name.
    pub name: &'static str,
    /// The byte offset of the field from the start of the containing type.
    pub offset: usize,
}

/// Structural metadata for a reflected type.
#[derive(Debug, Clone)]
pub struct Description {
    /// The user-facing name of the type.
    pub name: &'static str,
    /// The size of the type in bytes.
    pub size: usize,
    /// The structural classification of the type.
    pub value: Value,
}

/// A pairing of raw bytes with a reflected type description.
///
/// `Instance` currently stores the metadata needed to interpret a value, but the
/// module does not yet expose field accessors over the raw pointer.
#[derive(Debug, Clone)]
pub struct Instance {
    data: *const u8,
    size: usize,
    desc: Rc<Description>,
}

/// Associates arena-backed data with a registered description.
///
/// `key` is resolved through `registry`. If a description is found, an [`Instance`]
/// is created that points at the bytes owned by `data`.
pub fn introspect<T: Introspectable>(
    registry: &TypeRegistry,
    key: impl AsRef<str>,
    data: &Allocated<T>,
) -> Option<Instance> {
    registry.get(key).map(|desc| {
        let data = data.as_ref();
        Instance {
            data: data.as_ptr(),
            size: data.len(),
            desc: desc.clone(),
        }
    })
}
