//! JSON serialization helpers built on top of the reflection system.
//!
//! This module converts values that implement [`Introspectable`] into
//! [`json::JsonValue`] and reconstructs values from JSON using the same runtime
//! type description.
//!
//! The current implementation supports:
//! - scalar base types exposed through [`crate::reflect::Base`]
//! - composite types whose fields are described recursively
//!
//! The current implementation does not support:
//! - enumerations
//! - references
//!
//! Deserialization currently requires `T: Copy` because values are written into
//! an uninitialized output buffer and then returned by value.

use crate::reflect::{Base, Description, Introspectable, Value};
use crate::rust_alloc::string::String;
use json::JsonValue;
use core::mem::MaybeUninit;
use core::ptr;

/// Errors returned while serializing to or deserializing from JSON.
///
/// These errors distinguish between unsupported reflected shapes, structural
/// mismatches such as missing object fields, and numeric conversion failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The reflected value kind is not handled by the serializer.
    UnsupportedType(&'static str),
    /// A composite type expected a JSON object but received another JSON kind.
    ExpectedObject(&'static str),
    /// A required field was missing from the source object.
    MissingField(&'static str),
    /// A JSON number could not be interpreted as the requested numeric type.
    InvalidNumber {
        expected: &'static str,
        found: String,
    },
    /// A numeric value was parsed but does not fit in the destination type.
    OutOfRange {
        expected: &'static str,
        found: String,
    },
}

/// Serializes an introspectable value into JSON.
///
/// Scalars are encoded as JSON numbers or `null`, while composite values are
/// encoded as JSON objects whose keys match the reflected field names.
///
/// # Errors
///
/// Returns [`Error::UnsupportedType`] when the reflected type includes
/// unsupported constructs such as enumerations or references.
///
/// # Examples
///
/// ```
/// use foundation::reflect::{Base, Description, Field, Introspectable, Value};
/// use foundation::serializer;
///
/// #[repr(C)]
/// #[derive(Copy, Clone)]
/// struct Vec2 {
///     x: f32,
///     y: f32,
/// }
///
/// impl Introspectable for Vec2 {
///     fn description() -> Description {
///         Description {
///             name: "Vec2",
///             size: std::mem::size_of::<Vec2>(),
///             value: Value::Composite {
///                 fields: vec![
///                     Field {
///                         desc: Description {
///                             name: "f32",
///                             size: std::mem::size_of::<f32>(),
///                             value: Value::Scalar {
///                                 base: Base::Float32,
///                             },
///                         },
///                         name: "x",
///                         offset: std::mem::offset_of!(Vec2, x),
///                     },
///                     Field {
///                         desc: Description {
///                             name: "f32",
///                             size: std::mem::size_of::<f32>(),
///                             value: Value::Scalar {
///                                 base: Base::Float32,
///                             },
///                         },
///                         name: "y",
///                         offset: std::mem::offset_of!(Vec2, y),
///                     },
///                 ],
///             },
///         }
///     }
/// }
///
/// let encoded = serializer::serialize(&Vec2 { x: 1.5, y: -2.0 }).unwrap();
///
/// assert_eq!(encoded["x"], json::from(1.5));
/// assert_eq!(encoded["y"], json::from(-2.0));
/// ```
pub fn serialize<T: Introspectable>(value: &T) -> Result<JsonValue, Error> {
    let desc = T::description();
    let data = value as *const T as *const u8;
    serialize_desc(&desc, data)
}

/// Deserializes a JSON value into an introspectable Rust value.
///
/// The JSON structure must match the reflected description of `T`. Composite
/// types are read from JSON objects using the reflected field names.
///
/// `T` must implement [`Copy`] because the current implementation materializes
/// the output in a raw buffer and returns it by value.
///
/// # Errors
///
/// Returns:
/// - [`Error::ExpectedObject`] when a composite type is not backed by a JSON object
/// - [`Error::MissingField`] when a required field is absent
/// - [`Error::InvalidNumber`] when a JSON number cannot be read as the requested type
/// - [`Error::OutOfRange`] when the numeric value does not fit in the destination type
/// - [`Error::UnsupportedType`] for unsupported reflected shapes
///
/// # Examples
///
/// ```
/// use foundation::reflect::{Base, Description, Field, Introspectable, Value};
/// use foundation::serializer;
///
/// #[repr(C)]
/// #[derive(Debug, Copy, Clone, PartialEq)]
/// struct Entity {
///     id: u32,
///     health: i32,
/// }
///
/// impl Introspectable for Entity {
///     fn description() -> Description {
///         Description {
///             name: "Entity",
///             size: std::mem::size_of::<Entity>(),
///             value: Value::Composite {
///                 fields: vec![
///                     Field {
///                         desc: Description {
///                             name: "u32",
///                             size: std::mem::size_of::<u32>(),
///                             value: Value::Scalar {
///                                 base: Base::Unsigned32,
///                             },
///                         },
///                         name: "id",
///                         offset: std::mem::offset_of!(Entity, id),
///                     },
///                     Field {
///                         desc: Description {
///                             name: "i32",
///                             size: std::mem::size_of::<i32>(),
///                             value: Value::Scalar {
///                                 base: Base::Signed32,
///                             },
///                         },
///                         name: "health",
///                         offset: std::mem::offset_of!(Entity, health),
///                     },
///                 ],
///             },
///         }
///     }
/// }
///
/// let source = json::parse(r#"{ "id": 7, "health": -3 }"#).unwrap();
/// let entity: Entity = serializer::deserialize(&source).unwrap();
///
/// assert_eq!(entity, Entity { id: 7, health: -3 });
/// ```
pub fn deserialize<T: Introspectable + Copy>(value: &JsonValue) -> Result<T, Error> {
    let desc = T::description();
    let mut output = MaybeUninit::<T>::uninit();
    let out_ptr = output.as_mut_ptr() as *mut u8;

    unsafe { ptr::write_bytes(out_ptr, 0, core::mem::size_of::<T>()) };
    deserialize_desc(&desc, out_ptr, value)?;

    Ok(unsafe { output.assume_init() })
}

fn serialize_desc(desc: &Description, data: *const u8) -> Result<JsonValue, Error> {
    match &desc.value {
        Value::None => Ok(JsonValue::Null),
        Value::Scalar { base } => serialize_scalar(*base, data),
        Value::Composite { fields } => {
            let mut object = JsonValue::new_object();

            for field in fields {
                let field_ptr = unsafe { data.add(field.offset) };
                let field_value = serialize_desc(&field.desc, field_ptr)?;
                object
                    .insert(field.name, field_value)
                    .map_err(|_| Error::UnsupportedType("Unable to insert value into object"))?;
            }

            Ok(object)
        }
        Value::Enumeration { .. } => Err(Error::UnsupportedType(
            "Enumeration serialization is not currently supported",
        )),
        Value::Reference { .. } => Err(Error::UnsupportedType(
            "Reference serialization is not currently supported",
        )),
    }
}

fn deserialize_desc(desc: &Description, out: *mut u8, source: &JsonValue) -> Result<(), Error> {
    match &desc.value {
        Value::None => Ok(()),
        Value::Scalar { base } => deserialize_scalar(*base, out, source),
        Value::Composite { fields } => {
            let JsonValue::Object(object) = source else {
                return Err(Error::ExpectedObject(desc.name));
            };

            for field in fields {
                let Some(value) = object.get(field.name) else {
                    return Err(Error::MissingField(field.name));
                };

                let field_ptr = unsafe { out.add(field.offset) };
                deserialize_desc(&field.desc, field_ptr, value)?;
            }

            Ok(())
        }
        Value::Enumeration { .. } => Err(Error::UnsupportedType(
            "Enumeration deserialization is not currently supported",
        )),
        Value::Reference { .. } => Err(Error::UnsupportedType(
            "Reference deserialization is not currently supported",
        )),
    }
}

fn serialize_scalar(base: Base, data: *const u8) -> Result<JsonValue, Error> {
    let value = unsafe {
        match base {
            Base::Void => JsonValue::Null,
            Base::Unsigned8 => JsonValue::from(ptr::read_unaligned(data as *const u8) as u64),
            Base::Unsigned16 => JsonValue::from(ptr::read_unaligned(data as *const u16) as u64),
            Base::Unsigned32 => JsonValue::from(ptr::read_unaligned(data as *const u32) as u64),
            Base::Unsigned64 => JsonValue::from(ptr::read_unaligned(data as *const u64)),
            Base::UnsignedPtr => JsonValue::from(ptr::read_unaligned(data as *const usize) as u64),
            Base::Signed8 => JsonValue::from(ptr::read_unaligned(data as *const i8) as i64),
            Base::Signed16 => JsonValue::from(ptr::read_unaligned(data as *const i16) as i64),
            Base::Signed32 => JsonValue::from(ptr::read_unaligned(data as *const i32) as i64),
            Base::Signed64 => JsonValue::from(ptr::read_unaligned(data as *const i64)),
            Base::SignedPtr => JsonValue::from(ptr::read_unaligned(data as *const isize) as i64),
            Base::Float32 => JsonValue::from(ptr::read_unaligned(data as *const f32) as f64),
            Base::Float64 => JsonValue::from(ptr::read_unaligned(data as *const f64)),
        }
    };

    Ok(value)
}

fn deserialize_scalar(base: Base, out: *mut u8, source: &JsonValue) -> Result<(), Error> {
    unsafe {
        match base {
            Base::Void => {}
            Base::Unsigned8 => {
                *(out as *mut u8) =
                    u8::try_from(read_u64(source, "u8")?).map_err(|_| Error::OutOfRange {
                        expected: "u8",
                        found: source.dump(),
                    })?;
            }
            Base::Unsigned16 => {
                *(out as *mut u16) =
                    u16::try_from(read_u64(source, "u16")?).map_err(|_| Error::OutOfRange {
                        expected: "u16",
                        found: source.dump(),
                    })?;
            }
            Base::Unsigned32 => {
                *(out as *mut u32) =
                    u32::try_from(read_u64(source, "u32")?).map_err(|_| Error::OutOfRange {
                        expected: "u32",
                        found: source.dump(),
                    })?;
            }
            Base::Unsigned64 => {
                *(out as *mut u64) = read_u64(source, "u64")?;
            }
            Base::UnsignedPtr => {
                *(out as *mut usize) =
                    usize::try_from(read_u64(source, "usize")?).map_err(|_| Error::OutOfRange {
                        expected: "usize",
                        found: source.dump(),
                    })?;
            }
            Base::Signed8 => {
                *(out as *mut i8) =
                    i8::try_from(read_i64(source, "i8")?).map_err(|_| Error::OutOfRange {
                        expected: "i8",
                        found: source.dump(),
                    })?;
            }
            Base::Signed16 => {
                *(out as *mut i16) =
                    i16::try_from(read_i64(source, "i16")?).map_err(|_| Error::OutOfRange {
                        expected: "i16",
                        found: source.dump(),
                    })?;
            }
            Base::Signed32 => {
                *(out as *mut i32) =
                    i32::try_from(read_i64(source, "i32")?).map_err(|_| Error::OutOfRange {
                        expected: "i32",
                        found: source.dump(),
                    })?;
            }
            Base::Signed64 => {
                *(out as *mut i64) = read_i64(source, "i64")?;
            }
            Base::SignedPtr => {
                *(out as *mut isize) =
                    isize::try_from(read_i64(source, "isize")?).map_err(|_| Error::OutOfRange {
                        expected: "isize",
                        found: source.dump(),
                    })?;
            }
            Base::Float32 => {
                *(out as *mut f32) = read_f64(source, "f32")? as f32;
            }
            Base::Float64 => {
                *(out as *mut f64) = read_f64(source, "f64")?;
            }
        }
    }

    Ok(())
}

fn read_u64(value: &JsonValue, expected: &'static str) -> Result<u64, Error> {
    value.as_u64().ok_or_else(|| Error::InvalidNumber {
        expected,
        found: value.dump(),
    })
}

fn read_i64(value: &JsonValue, expected: &'static str) -> Result<i64, Error> {
    value.as_i64().ok_or_else(|| Error::InvalidNumber {
        expected,
        found: value.dump(),
    })
}

fn read_f64(value: &JsonValue, expected: &'static str) -> Result<f64, Error> {
    value.as_f64().ok_or_else(|| Error::InvalidNumber {
        expected,
        found: value.dump(),
    })
}
