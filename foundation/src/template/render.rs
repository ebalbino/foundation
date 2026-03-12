use super::{Bindings, Node, Template};
use crate::alloc::{StringBuilder, StringRef};
use crate::rust_alloc::vec;
use crate::rust_alloc::vec::Vec;
use core::fmt::Write;

use super::bindings::BindingValue;

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
            stack: vec![],
        }
    }

    fn current(&self) -> Option<&'a BindingValue> {
        self.stack.last().map(|frame| frame.value)
    }

    fn current_index(&self) -> Option<usize> {
        self.stack.last().and_then(|frame| frame.index)
    }

    fn push(&mut self, value: &'a BindingValue, index: usize) {
        self.stack.push(Frame {
            value,
            index: Some(index),
        });
    }

    fn pop(&mut self) {
        if !self.stack.is_empty() {
            let _ = self.stack.pop();
        }
    }
}

pub(super) fn render_template(
    template: &Template,
    builder: &mut StringBuilder,
    binding: &Bindings,
) -> Option<()> {
    let mut scope = Scope::new(binding);
    render_nodes(&template.nodes, builder, &mut scope)
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
        Resolved::Value(value) => match value {
            BindingValue::Null => {}
            BindingValue::Bool(v) => write!(builder, "{v}").ok()?,
            BindingValue::Integer(v) => write!(builder, "{v}").ok()?,
            BindingValue::Float(v) => write!(builder, "{v}").ok()?,
            BindingValue::String(v) => builder.append(v)?,
            BindingValue::List(_) | BindingValue::Table(_) => write!(builder, "{value}").ok()?,
        },
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
        BindingValue::String(v) => !v.is_empty(),
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
        let next = parts.next()?;
        scope.root.get(&StringRef::from(next))?
    } else if first == "this" {
        scope.current()?
    } else {
        scope
            .current()
            .and_then(|value| get_child(value, first))
            .or_else(|| scope.root.get(&StringRef::from(first)))?
    };

    for part in parts {
        current = get_child(current, part)?;
    }

    Some(Resolved::Value(current))
}

fn get_child<'a>(value: &'a BindingValue, part: &str) -> Option<&'a BindingValue> {
    match value {
        BindingValue::Table(entries) => entries.get(&StringRef::from(part)),
        BindingValue::List(items) => {
            let index = part.parse::<usize>().ok()?;
            items.get(index)
        }
        _ => None,
    }
}
