use foundation::encoding;

#[test]
fn json_parses_objects_and_arrays() {
    let value = encoding::json(r#"{ "name": "shrine", "count": 3, "items": [1, 2, 3] }"#).unwrap();

    assert_eq!(value["name"], "shrine");
    assert_eq!(value["count"], 3);
    assert_eq!(value["items"][0], 1);
    assert_eq!(value["items"][2], 3);
}

#[test]
fn json_returns_errors_for_invalid_input() {
    let result = encoding::json("{ invalid json }");

    assert!(result.is_err());
}

#[test]
fn yaml_parses_multiple_documents() {
    let docs = encoding::yaml(
        r#"
name: shrine
enabled: true
---
items:
  - first
  - second
"#,
    )
    .unwrap();

    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0]["name"].as_str(), Some("shrine"));
    assert_eq!(docs[0]["enabled"].as_bool(), Some(true));
    assert_eq!(docs[1]["items"][0].as_str(), Some("first"));
    assert_eq!(docs[1]["items"][1].as_str(), Some("second"));
}

#[test]
fn yaml_returns_errors_for_invalid_input() {
    let result = encoding::yaml("key: [unterminated");

    assert!(result.is_err());
}

#[test]
fn toml_parses_tables_and_scalars() {
    let table = encoding::toml(
        r#"
title = "shrine"

[server]
port = 8080
enabled = true
"#,
    )
    .unwrap();

    assert_eq!(table["title"].as_str(), Some("shrine"));
    assert_eq!(table["server"]["port"].as_integer(), Some(8080));
    assert_eq!(table["server"]["enabled"].as_bool(), Some(true));
}

#[test]
fn toml_returns_errors_for_invalid_input() {
    let result = encoding::toml("title = ");

    assert!(result.is_err());
}
