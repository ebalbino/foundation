use foundation::alloc::{arena, string_builder};
use std::fmt::Write;

#[test]
fn appends_and_builds_strings() {
    let arena = arena(1024);
    let mut builder = string_builder(arena, 32);

    builder.append("Hello,").unwrap();
    builder.append(" world!").unwrap();

    assert_eq!(&builder.build().unwrap(), "Hello, world!");
}

#[test]
fn write_macro_supports_large_multi_page_strings() {
    let arena = arena(4096);
    let mut builder = string_builder(arena, 128);
    let chunk = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.";

    write!(builder, "{chunk} {chunk} {chunk}").unwrap();

    assert_eq!(
        &builder.build().unwrap(),
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Lorem ipsum dolor sit amet, consectetur adipiscing elit."
    );
}

#[test]
fn clear_removes_previous_data() {
    let arena = arena(1024);
    let mut builder = string_builder(arena, 32);

    write!(builder, "alpha").unwrap();
    assert_eq!(&builder.build().unwrap(), "alpha");

    builder.clear();
    write!(builder, "beta").unwrap();

    assert_eq!(&builder.build().unwrap(), "beta");
}
