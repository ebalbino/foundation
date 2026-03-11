//! Lightweight helpers for parsing common text encodings.
//!
//! This module provides small wrapper functions around the workspace's JSON,
//! YAML, and TOML parser dependencies. Each function accepts any `&str`-like
//! input via `AsRef<str>` and returns the parser's native output and error
//! types without introducing additional abstraction.
//!
//! Use these helpers when you want a compact parsing entry point in examples,
//! tests, or application code.

use crate::rust_alloc::vec::Vec;
use json::JsonValue;
use toml::Table;
use yaml_rust2::yaml::{Yaml, YamlLoader};

/// Parses a JSON document into a [`json::JsonValue`].
///
/// This is a thin wrapper around [`json::parse`]. It accepts any string-like
/// input and returns the parsed JSON value on success.
///
/// # Errors
///
/// Returns [`json::Error`] when the input is not valid JSON.
///
/// # Examples
///
/// ```
/// use foundation::encoding;
///
/// let value = encoding::json(r#"{ "name": "shrine", "count": 3 }"#).unwrap();
///
/// assert_eq!(value["name"], "shrine");
/// assert_eq!(value["count"], 3);
/// ```
pub fn json(source: impl AsRef<str>) -> Result<JsonValue, json::Error> {
    json::parse(source.as_ref())
}

/// Parses one or more YAML documents into a [`Vec<Yaml>`].
///
/// YAML streams may contain multiple documents separated by `---`, so this
/// function returns a vector rather than a single value.
///
/// # Errors
///
/// Returns [`yaml_rust2::scanner::ScanError`] when the input is not valid
/// YAML.
///
/// # Examples
///
/// ```
/// use foundation::encoding;
///
/// let docs = encoding::yaml(
///     r#"
/// name: shrine
/// ---
/// enabled: true
/// "#,
/// )
/// .unwrap();
///
/// assert_eq!(docs.len(), 2);
/// assert_eq!(docs[0]["name"].as_str(), Some("shrine"));
/// assert_eq!(docs[1]["enabled"].as_bool(), Some(true));
/// ```
pub fn yaml(source: impl AsRef<str>) -> Result<Vec<Yaml>, yaml_rust2::scanner::ScanError> {
    YamlLoader::load_from_str(source.as_ref())
}

/// Parses a TOML document into a [`toml::Table`].
///
/// This helper parses the input directly as a TOML table, which matches the
/// top-level structure returned by the `toml` crate for document-style input.
///
/// # Errors
///
/// Returns [`toml::de::Error`] when the input is not valid TOML.
///
/// # Examples
///
/// ```
/// use foundation::encoding;
///
/// let table = encoding::toml(
///     r#"
/// title = "shrine"
///
/// [server]
/// port = 8080
/// "#,
/// )
/// .unwrap();
///
/// assert_eq!(table["title"].as_str(), Some("shrine"));
/// assert_eq!(table["server"]["port"].as_integer(), Some(8080));
/// ```
pub fn toml(source: impl AsRef<str>) -> Result<Table, toml::de::Error> {
    source.as_ref().parse::<Table>()
}
