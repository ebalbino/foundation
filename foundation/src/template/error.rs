use crate::alloc::string::{self, String};
use crate::alloc::Arena;
use crate::rust_alloc::rc::Rc;
use core::fmt;

/// Errors produced when loading or parsing templates.
#[derive(Debug, Clone)]
pub enum TemplateError {
    /// The arena could not allocate storage for template data.
    AllocationFailed,
    /// A filesystem error occurred while reading a template file.
    Io(String),
    /// The template source contained invalid syntax.
    Parse(String),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::AllocationFailed => write!(f, "Template allocation failed"),
            TemplateError::Io(message) => write!(f, "{message}"),
            TemplateError::Parse(message) => write!(f, "{message}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TemplateError {}

pub(crate) fn copy_string(arena: Rc<Arena>, value: &str) -> Result<String, TemplateError> {
    string::make(arena, value).ok_or(TemplateError::AllocationFailed)
}

pub(crate) fn parse_error(arena: Rc<Arena>, value: &str) -> TemplateError {
    match string::make(arena, value) {
        Some(message) => TemplateError::Parse(message),
        None => TemplateError::AllocationFailed,
    }
}

#[cfg(feature = "std")]
pub(crate) fn io_error(arena: Rc<Arena>, value: &str) -> TemplateError {
    match string::make(arena, value) {
        Some(message) => TemplateError::Io(message),
        None => TemplateError::AllocationFailed,
    }
}
