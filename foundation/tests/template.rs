use foundation::alloc;
use foundation::template::{BindingValue, Bindings, Template};

fn bindings(entries: impl IntoIterator<Item = (&'static str, BindingValue)>) -> Bindings {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

#[test]
fn render_simple_bindings() {
    let arena = alloc::arena(1024 * 1024);
    let data = bindings([
        ("name", BindingValue::String("shrine".to_owned())),
        ("version", BindingValue::Integer(3)),
    ]);
    let template = Template::parse("project={{name}} v{{version}}").unwrap();

    let rendered = template.render_string(&arena, 256, &data).unwrap();
    assert_eq!(&rendered, "project=shrine v3");
}

#[test]
fn render_conditionals() {
    let arena = alloc::arena(1024 * 1024);
    let enabled = bindings([("enabled", BindingValue::Bool(true))]);
    let disabled = bindings([("enabled", BindingValue::Bool(false))]);
    let template = Template::parse("{{#if enabled}}on{{else}}off{{/if}}").unwrap();

    let on = template.render_string(&arena, 256, &enabled).unwrap();
    let off = template.render_string(&arena, 256, &disabled).unwrap();

    assert_eq!(&on, "on");
    assert_eq!(&off, "off");
}

#[test]
fn render_each_with_index_and_this() {
    let arena = alloc::arena(1024 * 1024);
    let data = bindings([(
        "items",
        BindingValue::List(vec![
            BindingValue::String("red".to_owned()),
            BindingValue::String("green".to_owned()),
            BindingValue::String("blue".to_owned()),
        ]),
    )]);
    let template = Template::parse("{{#each items}}{{@index}}={{this}};{{/each}}").unwrap();

    let rendered = template.render_string(&arena, 256, &data).unwrap();
    assert_eq!(&rendered, "0=red;1=green;2=blue;");
}

#[test]
fn render_nested_if_inside_each() {
    let arena = alloc::arena(1024 * 1024);
    let data = bindings([(
        "users",
        BindingValue::List(vec![
            BindingValue::Table(bindings([
                ("name", BindingValue::String("ana".to_owned())),
                ("admin", BindingValue::Bool(true)),
            ])),
            BindingValue::Table(bindings([
                ("name", BindingValue::String("bob".to_owned())),
                ("admin", BindingValue::Bool(false)),
            ])),
        ]),
    )]);

    let template =
        Template::parse("{{#each users}}{{name}}{{#if admin}}*{{/if}},{{/each}}").unwrap();

    let rendered = template.render_string(&arena, 256, &data).unwrap();
    assert_eq!(&rendered, "ana*,bob,");
}
