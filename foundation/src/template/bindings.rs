use crate::alloc::string::String;
use crate::alloc::StringRef;
use crate::rust_alloc::collections::BTreeMap;
use crate::rust_alloc::vec::Vec;
use core::fmt;

/// Table type used for template bindings.
pub type Bindings = BTreeMap<StringRef, BindingValue>;

/// Template value model used during path lookup and rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<BindingValue>),
    Table(Bindings),
}

impl fmt::Display for BindingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BindingValue::Null => write!(f, "null"),
            BindingValue::Bool(v) => write!(f, "{v}"),
            BindingValue::Integer(v) => write!(f, "{v}"),
            BindingValue::Float(v) => write!(f, "{v}"),
            BindingValue::String(v) => write!(f, "{v}"),
            BindingValue::List(values) => {
                write!(f, "[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{value}")?;
                }
                write!(f, "]")
            }
            BindingValue::Table(entries) => {
                write!(f, "{{")?;
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{key}:{value}")?;
                }
                write!(f, "}}")
            }
        }
    }
}
