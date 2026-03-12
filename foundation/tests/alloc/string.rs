use foundation::alloc::{arena, string, String};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[test]
fn make_and_wrap_preserve_contents() {
    let arena = arena(256);
    let string = string::make(arena.clone(), "Hello, world!").unwrap();
    let mut buffer = arena.allocate::<u8>("Hello, world!".len()).unwrap();
    buffer.copy_from_slice(b"Hello, world!");
    let wrapped = String::from(buffer);

    assert_eq!(&string, "Hello, world!");
    assert_eq!(&wrapped, "Hello, world!");
}

#[test]
fn clone_and_duplicate_manage_storage_distinctly() {
    let arena = arena(256);
    let string = string::make(arena.clone(), "hello").unwrap();

    assert_eq!(arena.current_position(), 5);
    assert_eq!(Rc::weak_count(&arena), 1);

    let clone = string.clone();
    assert_eq!(arena.current_position(), 5);
    assert_eq!(Rc::weak_count(&arena), 2);
    assert_eq!(string.as_ptr(), clone.as_ptr());

    let duplicate = string::duplicate(&string).unwrap();
    assert_eq!(arena.current_position(), 10);
    assert_eq!(&duplicate, "hello");
    assert_ne!(string.as_ptr(), duplicate.as_ptr());
}

#[test]
fn borrowed_string_ref_matches_hash_and_comparisons() {
    let arena = arena(256);
    let string = string::make(arena.clone(), "borrowed").unwrap();
    let borrowed = string.borrow();
    let literal = "borrowed";

    let mut string_hash = DefaultHasher::new();
    borrowed.hash(&mut string_hash);

    let mut literal_hash = DefaultHasher::new();
    literal.hash(&mut literal_hash);

    assert_eq!(&borrowed, "borrowed");
    assert_eq!(&borrowed, "borrowed");
    assert_eq!(borrowed, b"borrowed"[..]);
    assert_eq!(borrowed, string);
    assert_eq!(string_hash.finish(), literal_hash.finish());
}
