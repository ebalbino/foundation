use foundation::alloc::{arena, string_pool};

#[test]
fn intern_reuses_existing_strings() {
    let arena = arena(256);
    let mut pool = string_pool(arena);

    let first = pool.intern("foo").unwrap();
    let second = pool.intern("bar").unwrap();
    let first_again = pool.intern("foo").unwrap();
    let second_again = pool.intern("bar").unwrap();

    assert_eq!(first, first_again);
    assert_eq!(second, second_again);
    assert_eq!(first.as_ptr(), first_again.as_ptr());
    assert_eq!(second.as_ptr(), second_again.as_ptr());
}

#[test]
fn get_returns_interned_values() {
    let arena = arena(256);
    let mut pool = string_pool(arena);

    pool.intern("baz").unwrap();

    assert_eq!(pool.get("baz").map(|value| &**value), Some("baz"));
    assert!(pool.get("missing").is_none());
}
