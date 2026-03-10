use foundation::alloc::{arena, buffer_builder};
use std::io::Write;

#[test]
fn builds_from_multiple_pages() {
    let arena = arena(256);
    let mut builder = buffer_builder(arena, 8);

    builder.append([1, 2, 3, 4, 5]).unwrap();
    builder.append([6, 7, 8, 9, 10]).unwrap();

    let buffer = builder.build().unwrap();
    assert_eq!(&buffer[..], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn clear_discards_previous_contents() {
    let arena = arena(256);
    let mut builder = buffer_builder(arena, 16);

    builder.append(b"hello").unwrap();

    let bytes = builder.build().unwrap();
    assert_eq!(&bytes[..], b"hello");

    builder.clear();
    builder.append(b"bye").unwrap();

    let bytes = builder.build().unwrap();
    assert_eq!(&bytes[..], b"bye");
}

#[test]
fn write_trait_appends_bytes() {
    let arena = arena(256);
    let mut builder = buffer_builder(arena, 8);

    builder.write_all(&[1, 2, 3, 4]).unwrap();
    builder.write_all(&[5, 6]).unwrap();

    let bytes = builder.build().unwrap();

    assert_eq!(&bytes[..], &[1, 2, 3, 4, 5, 6]);
}
