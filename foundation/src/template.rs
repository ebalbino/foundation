//! A small table-backed template renderer.
//!
//! Templates are parsed into a compact node tree and rendered against a
//! [`Bindings`] table. The syntax is intentionally minimal:
//!
//! - `{{path}}` evaluates and renders a value
//! - `{{#if path}}...{{else}}...{{/if}}` renders conditionally
//! - `{{#each path}}...{{/each}}` iterates arrays and objects
//!
//! During rendering, path lookup supports:
//!
//! - `this` for the current loop item
//! - `@index` for the current loop index
//! - `$root` to force lookup from the root binding
//! - dotted access such as `user.name` or `items.0`
//!
//! Missing values render as empty output. Truthiness follows simple data-model
//! rules: `null`, `false`, `0`, empty strings, empty arrays, and empty tables
//! are falsey.

mod bindings;
mod error;
mod parser;
mod render;

use crate::alloc::string::String;
use crate::alloc::{Arena, StringBuilder, string_builder};
#[cfg(feature = "std")]
use crate::file;
use crate::rust_alloc::rc::Rc;
use crate::rust_alloc::vec::Vec;
#[cfg(feature = "std")]
use std::path::Path;

pub use bindings::{BindingValue, Bindings};
pub use error::TemplateError;

/// A parsed template ready to be rendered against table data.
#[derive(Debug, Clone)]
pub struct Template {
    nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
enum Node {
    Text(String),
    Eval(String),
    If {
        condition: String,
        then_nodes: Vec<Node>,
        else_nodes: Vec<Node>,
    },
    Each {
        binding: String,
        body: Vec<Node>,
    },
}

#[derive(Debug, Clone)]
enum Token {
    Text(String),
    Tag(Tag),
}

#[derive(Debug, Clone)]
enum Tag {
    Eval(String),
    IfStart(String),
    Else,
    IfEnd,
    EachStart(String),
    EachEnd,
}

#[derive(Debug, Clone, Copy)]
enum StopTag {
    Else,
    IfEnd,
    EachEnd,
}

impl Template {
    /// Parses template source into a [`Template`].
    ///
    /// Parsing validates control-tag structure such as matching `{{#if}}` /
    /// `{{/if}}` and `{{#each}}` / `{{/each}}` pairs.
    pub fn parse(arena: Rc<Arena>, source: impl AsRef<str>) -> Result<Self, TemplateError> {
        parser::parse_template(arena, source.as_ref())
    }

    /// Loads a template from disk and parses it.
    #[cfg(feature = "std")]
    pub fn load(arena: Rc<Arena>, path: impl AsRef<Path>) -> Result<Self, TemplateError> {
        let bytes = file::load(arena, path.as_ref()).ok_or_else(|| {
            error::io_error(
                arena,
                &std::format!("Unable to load template {}", path.as_ref().display()),
            )
        })?;
        let source = String::from(bytes);

        Self::parse(arena, &*source)
    }

    /// Renders the template into an existing [`StringBuilder`].
    ///
    /// Returns `None` if the builder cannot allocate additional space while
    /// rendering.
    pub fn render(&self, builder: &mut StringBuilder, binding: &Bindings) -> Option<()> {
        render::render_template(self, builder, binding)
    }

    /// Renders the template into a newly created arena-backed [`String`].
    ///
    /// `page_size` controls the internal page size used by the temporary
    /// [`StringBuilder`].
    pub fn render_string(
        &self,
        arena: Rc<Arena>,
        page_size: usize,
        binding: &Bindings,
    ) -> Option<String> {
        let mut builder = string_builder(arena, page_size);
        self.render(&mut builder, binding)?;
        builder.build()
    }
}
