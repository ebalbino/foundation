use foundation::alloc;
use foundation::template::Template;

#[test]
fn render_simple_bindings() {
    let arena = alloc::arena(1024 * 1024);
    let data = json::parse(r#"{ "name": "shrine", "version": 3 }"#).unwrap();
    let template = Template::parse("project={{name}} v{{version}}").unwrap();

    let rendered = template.render_string(&arena, 256, &data).unwrap();
    assert_eq!(&rendered, "project=shrine v3");
}

#[test]
fn render_conditionals() {
    let arena = alloc::arena(1024 * 1024);
    let enabled = json::parse(r#"{ "enabled": true }"#).unwrap();
    let disabled = json::parse(r#"{ "enabled": false }"#).unwrap();
    let template = Template::parse("{{#if enabled}}on{{else}}off{{/if}}").unwrap();

    let on = template.render_string(&arena, 256, &enabled).unwrap();
    let off = template.render_string(&arena, 256, &disabled).unwrap();

    assert_eq!(&on, "on");
    assert_eq!(&off, "off");
}

#[test]
fn render_each_with_index_and_this() {
    let arena = alloc::arena(1024 * 1024);
    let data = json::parse(r#"{ "items": ["red", "green", "blue"] }"#).unwrap();
    let template = Template::parse("{{#each items}}{{@index}}={{this}};{{/each}}").unwrap();

    let rendered = template.render_string(&arena, 256, &data).unwrap();
    assert_eq!(&rendered, "0=red;1=green;2=blue;");
}

#[test]
fn render_nested_if_inside_each() {
    let arena = alloc::arena(1024 * 1024);
    let data = json::parse(
        r#"
        {
          "users": [
            { "name": "ana", "admin": true },
            { "name": "bob", "admin": false }
          ]
        }
        "#,
    )
    .unwrap();

    let template =
        Template::parse("{{#each users}}{{name}}{{#if admin}}*{{/if}},{{/each}}").unwrap();

    let rendered = template.render_string(&arena, 256, &data).unwrap();
    assert_eq!(&rendered, "ana*,bob,");
}
