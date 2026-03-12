use foundation::alloc;
use foundation::alloc::string;
use foundation::alloc::{String, StringRef};
use foundation::template::{BindingValue, Bindings, Template};

fn arena_string(arena: std::rc::Rc<foundation::alloc::Arena>, value: &str) -> String {
    string::make(arena, value).unwrap()
}

fn key(arena: std::rc::Rc<foundation::alloc::Arena>, value: &str) -> StringRef {
    arena_string(arena, value).borrow()
}

fn bindings(
    arena: &std::rc::Rc<foundation::alloc::Arena>,
    entries: impl IntoIterator<Item = (&'static str, BindingValue)>,
) -> Bindings {
    entries
        .into_iter()
        .map(|(key_name, value)| (key(arena.clone(), key_name), value))
        .collect()
}

#[test]
fn render_simple_bindings() {
    let arena = alloc::arena(1024 * 1024);
    let data = bindings(&arena, [
        ("name", BindingValue::String(arena_string(arena.clone(), "shrine"))),
        ("version", BindingValue::Integer(3)),
    ]);
    let template = Template::parse(arena.clone(), "project={{name}} v{{version}}").unwrap();

    let rendered = template.render_string(arena.clone(), 256, &data).unwrap();
    assert_eq!(&rendered, "project=shrine v3");
}

#[test]
fn render_conditionals() {
    let arena = alloc::arena(1024 * 1024);
    let enabled = bindings(&arena, [("enabled", BindingValue::Bool(true))]);
    let disabled = bindings(&arena, [("enabled", BindingValue::Bool(false))]);
    let template = Template::parse(arena.clone(), "{{#if enabled}}on{{else}}off{{/if}}").unwrap();

    let on = template.render_string(arena.clone(), 256, &enabled).unwrap();
    let off = template.render_string(arena.clone(), 256, &disabled).unwrap();

    assert_eq!(&on, "on");
    assert_eq!(&off, "off");
}

#[test]
fn render_each_with_index_and_this() {
    let arena = alloc::arena(1024 * 1024);
    let data = bindings(&arena, [(
        "items",
        BindingValue::List(vec![
            BindingValue::String(arena_string(arena.clone(), "red")),
            BindingValue::String(arena_string(arena.clone(), "green")),
            BindingValue::String(arena_string(arena.clone(), "blue")),
        ]),
    )]);
    let template = Template::parse(arena.clone(), "{{#each items}}{{@index}}={{this}};{{/each}}").unwrap();

    let rendered = template.render_string(arena.clone(), 256, &data).unwrap();
    assert_eq!(&rendered, "0=red;1=green;2=blue;");
}

#[test]
fn render_nested_if_inside_each() {
    let arena = alloc::arena(1024 * 1024);
    let data = bindings(&arena, [(
        "users",
        BindingValue::List(vec![
            BindingValue::Table(bindings(&arena, [
                ("name", BindingValue::String(arena_string(arena.clone(), "ana"))),
                ("admin", BindingValue::Bool(true)),
            ])),
            BindingValue::Table(bindings(&arena, [
                ("name", BindingValue::String(arena_string(arena.clone(), "bob"))),
                ("admin", BindingValue::Bool(false)),
            ])),
        ]),
    )]);

    let template = Template::parse(arena.clone(), "{{#each users}}{{name}}{{#if admin}}*{{/if}},{{/each}}").unwrap();

    let rendered = template.render_string(arena.clone(), 256, &data).unwrap();
    assert_eq!(&rendered, "ana*,bob,");
}
