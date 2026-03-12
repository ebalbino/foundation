use super::{Node, StopTag, Tag, Template, Token};
use super::error::{TemplateError, copy_string, parse_error};
use crate::alloc::Arena;
use crate::rust_alloc::format;
use crate::rust_alloc::rc::Rc;
use crate::rust_alloc::vec::Vec;

pub(super) fn parse_template(
    arena: Rc<Arena>,
    source: &str,
) -> Result<Template, TemplateError> {
    let tokens = tokenize(arena.clone(), source)?;
    let mut cursor = 0;
    let (nodes, stop) = parse_nodes(arena.clone(), &tokens, &mut cursor, &[])?;

    if stop.is_some() {
        return Err(parse_error(
            arena.clone(),
            "Unexpected closing control tag at top-level",
        ));
    }

    Ok(Template { nodes })
}

fn tokenize(arena: Rc<Arena>, source: &str) -> Result<Vec<Token>, TemplateError> {
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while let Some(relative_open) = source[cursor..].find("{{") {
        let open = cursor + relative_open;
        if open > cursor {
            tokens.push(Token::Text(copy_string(arena.clone(), &source[cursor..open])?));
        }

        let tag_start = open + 2;
        let Some(relative_close) = source[tag_start..].find("}}") else {
            return Err(parse_error(arena.clone(), &format!("Unclosed tag near byte {open}")));
        };

        let close = tag_start + relative_close;
        let raw = source[tag_start..close].trim();
        tokens.push(Token::Tag(parse_tag(&arena, raw)?));
        cursor = close + 2;
    }

    if cursor < source.len() {
        tokens.push(Token::Text(copy_string(arena.clone(), &source[cursor..])?));
    }

    Ok(tokens)
}

fn parse_tag(arena: &Rc<Arena>, raw: &str) -> Result<Tag, TemplateError> {
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
            return Err(parse_error(arena.clone(), "Missing condition in if tag"));
        }
        return Ok(Tag::IfStart(copy_string(arena.clone(), condition)?));
    }

    if let Some(rest) = raw.strip_prefix("#each") {
        let binding = rest.trim();
        if binding.is_empty() {
            return Err(parse_error(arena.clone(), "Missing binding in each tag"));
        }
        return Ok(Tag::EachStart(copy_string(arena.clone(), binding)?));
    }

    if raw.is_empty() {
        return Err(parse_error(arena.clone(), "Empty tag is not allowed"));
    }

    Ok(Tag::Eval(copy_string(arena.clone(), raw)?))
}

fn parse_nodes(
    arena: Rc<Arena>,
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
                        parse_nodes(arena.clone(), tokens, cursor, &[StopTag::Else, StopTag::IfEnd])?;
                    let else_nodes = match stop {
                        Some(StopTag::Else) => {
                            let (branch, stop) =
                                parse_nodes(arena.clone(), tokens, cursor, &[StopTag::IfEnd])?;
                            if !matches!(stop, Some(StopTag::IfEnd)) {
                                return Err(parse_error(arena.clone(), "Missing {{/if}} after {{else}}"));
                            }
                            branch
                        }
                        Some(StopTag::IfEnd) => Vec::new(),
                        _ => {
                            return Err(parse_error(
                                arena,
                                "Missing {{/if}} for conditional block",
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
                    let (body, stop) = parse_nodes(arena.clone(), tokens, cursor, &[StopTag::EachEnd])?;
                    if !matches!(stop, Some(StopTag::EachEnd)) {
                        return Err(parse_error(arena.clone(), "Missing {{/each}} for loop block"));
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

                    return Err(parse_error(arena.clone(), "Unexpected {{else}} outside if block"));
                }
                Tag::IfEnd => {
                    if stop_tags.iter().any(|stop| matches!(stop, StopTag::IfEnd)) {
                        *cursor += 1;
                        return Ok((nodes, Some(StopTag::IfEnd)));
                    }

                    return Err(parse_error(arena.clone(), "Unexpected {{/if}} outside if block"));
                }
                Tag::EachEnd => {
                    if stop_tags.iter().any(|stop| matches!(stop, StopTag::EachEnd)) {
                        *cursor += 1;
                        return Ok((nodes, Some(StopTag::EachEnd)));
                    }

                    return Err(parse_error(arena.clone(), "Unexpected {{/each}} outside each block"));
                }
            },
        }
    }

    Ok((nodes, None))
}
