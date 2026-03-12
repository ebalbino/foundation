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

use crate::alloc::string::String;
use crate::alloc::{Arena, StringBuilder, string_builder};
use crate::rust_alloc::collections::BTreeMap;
use crate::rust_alloc::borrow::ToOwned;
use crate::rust_alloc::format;
use crate::rust_alloc::rc::Rc;
use crate::rust_alloc::string::String as StdString;
use crate::rust_alloc::vec::Vec;
use crate::alloc::String;
#[cfg(feature = "std")]
use crate::file;
use core::fmt::{self, Write};
#[cfg(feature = "std")]
use std::path::Path;

/// Table type used for template bindings.
pub type Bindings = BTreeMap<StdString, BindingValue>;

/// Template value model used during path lookup and rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(StdString),
    List(Vec<BindingValue>),
    Table(Bindings),
}

/// A parsed template ready to be rendered against table data.
#[derive(Debug, Clone)]
pub struct Template {
    nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
enum Node {
    Text(StdString),
    Eval(StdString),
    If {
        condition: StdString,
        then_nodes: Vec<Node>,
        else_nodes: Vec<Node>,
    },
    Each {
        binding: StdString,
        body: Vec<Node>,
    },
}

/// Errors produced when loading or parsing templates.
#[derive(Debug, Clone)]
pub enum TemplateError {
    /// A filesystem error occurred while reading a template file.
    Io(StdString),
    /// The template source contained invalid syntax.
    Parse(StdString),
}

impl Template {
    /// Parses template source into a [`Template`].
    ///
    /// Parsing validates control-tag structure such as matching `{{#if}}` /
    /// `{{/if}}` and `{{#each}}` / `{{/each}}` pairs.
    pub fn parse(source: impl AsRef<str>) -> Result<Self, TemplateError> {
        let tokens = tokenize(source.as_ref())?;
        let mut cursor = 0;
        let (nodes, stop) = parse_nodes(&tokens, &mut cursor, &[])?;

        if stop.is_some() {
            return Err(TemplateError::Parse(
                "Unexpected closing control tag at top-level".to_owned(),
            ));
        }

        Ok(Self { nodes })
    }

    /// Loads a template from disk and parses it.
    #[cfg(feature = "std")]
    pub fn load(path: impl AsRef<Path>) -> Result<Self, TemplateError> {
        let source = file::load(path.as_ref()).map(String::from).map_err(|e| {
            TemplateError::Io(format!(
                "Unable to load template {}: {e}",
                path.as_ref().display()
            ))
        })?;

        Self::parse(source)
    }

    /// Renders the template into an existing [`StringBuilder`].
    ///
    /// Returns `None` if the builder cannot allocate additional space while
    /// rendering.
    pub fn render(&self, builder: &mut StringBuilder, binding: &Bindings) -> Option<()> {
        let mut scope = Scope::new(binding);
        render_nodes(&self.nodes, builder, &mut scope)
    }

    /// Renders the template into a newly created arena-backed [`String`].
    ///
    /// `page_size` controls the internal page size used by the temporary
    /// [`StringBuilder`].
    pub fn render_string(
        &self,
        arena: &Rc<Arena>,
        page_size: usize,
        binding: &Bindings,
    ) -> Option<String> {
        let mut builder = string_builder(arena.clone(), page_size);
        self.render(&mut builder, binding)?;
        builder.build()
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::Io(message) => write!(f, "{message}"),
            TemplateError::Parse(message) => write!(f, "{message}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TemplateError {}

#[derive(Debug, Clone)]
enum Token {
    Text(StdString),
    Tag(Tag),
}

#[derive(Debug, Clone)]
enum Tag {
    Eval(StdString),
    IfStart(StdString),
    Else,
    IfEnd,
    EachStart(StdString),
    EachEnd,
}

#[derive(Debug, Clone, Copy)]
enum StopTag {
    Else,
    IfEnd,
    EachEnd,
}

#[derive(Debug, Clone)]
enum Resolved<'a> {
    Value(&'a BindingValue),
    Index(usize),
}

#[derive(Debug, Clone, Copy)]
struct Frame<'a> {
    value: &'a BindingValue,
    index: Option<usize>,
}

#[derive(Debug, Clone)]
struct Scope<'a> {
    root: &'a Bindings,
    stack: Vec<Frame<'a>>,
}

impl<'a> Scope<'a> {
    fn new(root: &'a Bindings) -> Self {
        Self {
            root,
            stack: Vec::new(),
        }
    }

    fn current(&self) -> Option<&'a BindingValue> {
        self.stack.last().map(|f| f.value)
    }

    fn current_index(&self) -> Option<usize> {
        self.stack.last().and_then(|f| f.index)
    }

    fn push(&mut self, value: &'a BindingValue, index: usize) {
        self.stack.push(Frame {
            value,
            index: Some(index),
        });
    }

    fn pop(&mut self) {
        if self.stack.len() > 1 {
            let _ = self.stack.pop();
        }
    }
}

fn tokenize(source: &str) -> Result<Vec<Token>, TemplateError> {
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while let Some(relative_open) = source[cursor..].find("{{") {
        let open = cursor + relative_open;
        if open > cursor {
            tokens.push(Token::Text(source[cursor..open].to_owned()));
        }

        let tag_start = open + 2;
        let Some(relative_close) = source[tag_start..].find("}}") else {
            return Err(TemplateError::Parse(format!(
                "Unclosed tag near byte {open}"
            )));
        };

        let close = tag_start + relative_close;
        let raw = source[tag_start..close].trim();
        tokens.push(Token::Tag(parse_tag(raw)?));
        cursor = close + 2;
    }

    if cursor < source.len() {
        tokens.push(Token::Text(source[cursor..].to_owned()));
    }

    Ok(tokens)
}

fn parse_tag(raw: &str) -> Result<Tag, TemplateError> {
    if raw == "else" {
        return Ok(Tag::Else);
    }

    if raw == "/if" {
        return Ok(Tag::IfEnd);
    }

    if raw == "/each" {
        return Ok(Tag::EachEnd);
    }

    if let Some(rest) = raw.strip_prefix("#if") {
        let condition = rest.trim();
        if condition.is_empty() {
            return Err(TemplateError::Parse(
                "Missing condition in if tag".to_owned(),
            ));
        }
        return Ok(Tag::IfStart(condition.to_owned()));
    }

    if let Some(rest) = raw.strip_prefix("#each") {
        let binding = rest.trim();
        if binding.is_empty() {
            return Err(TemplateError::Parse(
                "Missing binding in each tag".to_owned(),
            ));
        }
        return Ok(Tag::EachStart(binding.to_owned()));
    }

    if raw.is_empty() {
        return Err(TemplateError::Parse("Empty tag is not allowed".to_owned()));
    }

    Ok(Tag::Eval(raw.to_owned()))
}

fn parse_nodes(
    tokens: &[Token],
    cursor: &mut usize,
    stop_tags: &[StopTag],
) -> Result<(Vec<Node>, Option<StopTag>), TemplateError> {
    let mut nodes = Vec::new();

    while *cursor < tokens.len() {
        match &tokens[*cursor] {
            Token::Text(text) => {
                nodes.push(Node::Text(text.clone()));
                *cursor += 1;
            }
            Token::Tag(tag) => match tag {
                Tag::Eval(path) => {
                    nodes.push(Node::Eval(path.clone()));
                    *cursor += 1;
                }
                Tag::IfStart(condition) => {
                    *cursor += 1;
                    let (then_nodes, stop) =
                        parse_nodes(tokens, cursor, &[StopTag::Else, StopTag::IfEnd])?;
                    let else_nodes = match stop {
                        Some(StopTag::Else) => {
                            let (branch, stop) = parse_nodes(tokens, cursor, &[StopTag::IfEnd])?;
                            if !matches!(stop, Some(StopTag::IfEnd)) {
                                return Err(TemplateError::Parse(
                                    "Missing {{/if}} after {{else}}".to_owned(),
                                ));
                            }
                            branch
                        }
                        Some(StopTag::IfEnd) => Vec::new(),
                        _ => {
                            return Err(TemplateError::Parse(
                                "Missing {{/if}} for conditional block".to_owned(),
                            ));
                        }
                    };

                    nodes.push(Node::If {
                        condition: condition.clone(),
                        then_nodes,
                        else_nodes,
                    });
                }
                Tag::EachStart(binding) => {
                    *cursor += 1;
                    let (body, stop) = parse_nodes(tokens, cursor, &[StopTag::EachEnd])?;
                    if !matches!(stop, Some(StopTag::EachEnd)) {
                        return Err(TemplateError::Parse(
                            "Missing {{/each}} for loop block".to_owned(),
                        ));
                    }

                    nodes.push(Node::Each {
                        binding: binding.clone(),
                        body,
                    });
                }
                Tag::Else => {
                    if stop_tags.iter().any(|stop| matches!(stop, StopTag::Else)) {
                        *cursor += 1;
                        return Ok((nodes, Some(StopTag::Else)));
                    }

                    return Err(TemplateError::Parse(
                        "Unexpected {{else}} outside if block".to_owned(),
                    ));
                }
                Tag::IfEnd => {
                    if stop_tags.iter().any(|stop| matches!(stop, StopTag::IfEnd)) {
                        *cursor += 1;
                        return Ok((nodes, Some(StopTag::IfEnd)));
                    }

                    return Err(TemplateError::Parse(
                        "Unexpected {{/if}} outside if block".to_owned(),
                    ));
                }
                Tag::EachEnd => {
                    if stop_tags
                        .iter()
                        .any(|stop| matches!(stop, StopTag::EachEnd))
                    {
                        *cursor += 1;
                        return Ok((nodes, Some(StopTag::EachEnd)));
                    }

                    return Err(TemplateError::Parse(
                        "Unexpected {{/each}} outside each block".to_owned(),
                    ));
                }
            },
        }
    }

    Ok((nodes, None))
}

fn render_nodes(nodes: &[Node], builder: &mut StringBuilder, scope: &mut Scope<'_>) -> Option<()> {
    for node in nodes {
        match node {
            Node::Text(text) => builder.append(text)?,
            Node::Eval(path) => render_eval(path, builder, scope)?,
            Node::If {
                condition,
                then_nodes,
                else_nodes,
            } => {
                if expression_truthy(condition, scope) {
                    render_nodes(then_nodes, builder, scope)?;
                } else {
                    render_nodes(else_nodes, builder, scope)?;
                }
            }
            Node::Each { binding, body } => {
                if let Some(Resolved::Value(value)) = resolve(binding, scope) {
                    match value {
                        BindingValue::List(items) => {
                            for (index, item) in items.iter().enumerate() {
                                scope.push(item, index);
                                render_nodes(body, builder, scope)?;
                                scope.pop();
                            }
                        }
                        BindingValue::Table(entries) => {
                            for (index, (_, value)) in entries.iter().enumerate() {
                                scope.push(value, index);
                                render_nodes(body, builder, scope)?;
                                scope.pop();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Some(())
}

fn render_eval(path: &str, builder: &mut StringBuilder, scope: &Scope<'_>) -> Option<()> {
    let Some(value) = resolve(path, scope) else {
        return Some(());
    };

    match value {
        Resolved::Index(index) => write!(builder, "{index}").ok()?,
        Resolved::Value(value) => {
            match value {
                BindingValue::Null => {}
                BindingValue::Bool(v) => write!(builder, "{v}").ok()?,
                BindingValue::Integer(v) => write!(builder, "{v}").ok()?,
                BindingValue::Float(v) => write!(builder, "{v}").ok()?,
                BindingValue::String(v) => builder.append(v)?,
                BindingValue::List(_) | BindingValue::Table(_) => write!(builder, "{value}").ok()?,
            }
        }
    }

    Some(())
}

fn expression_truthy(path: &str, scope: &Scope<'_>) -> bool {
    match resolve(path, scope) {
        Some(Resolved::Index(index)) => index != 0,
        Some(Resolved::Value(value)) => value_truthy(value),
        None => false,
    }
}

fn value_truthy(value: &BindingValue) -> bool {
    match value {
        BindingValue::Null => false,
        BindingValue::Bool(v) => *v,
        BindingValue::Integer(v) => *v != 0,
        BindingValue::Float(v) => *v != 0.0,
        BindingValue::String(s) => !s.is_empty(),
        BindingValue::List(values) => !values.is_empty(),
        BindingValue::Table(entries) => !entries.is_empty(),
    }
}

fn resolve<'a>(path: &str, scope: &Scope<'a>) -> Option<Resolved<'a>> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }

    if path == "@index" {
        return scope.current_index().map(Resolved::Index);
    }

    let mut parts = path.split('.').filter(|part| !part.is_empty());
    let first = parts.next()?;

    let mut current = if first == "$root" {
        let first = parts.next()?;
        scope.root.get(first)?
    } else if first == "this" {
        scope.current()?
    } else {
        scope
            .current()
            .and_then(|value| get_child(value, first))
            .or_else(|| scope.root.get(first))?
    };

    for part in parts {
        current = get_child(current, part)?;
    }

    Some(Resolved::Value(current))
}

fn get_child<'a>(value: &'a BindingValue, part: &str) -> Option<&'a BindingValue> {
    match value {
        BindingValue::Table(entries) => entries.get(part),
        BindingValue::List(items) => {
            let index = part.parse::<usize>().ok()?;
            items.get(index)
        }
        _ => None,
    }
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
