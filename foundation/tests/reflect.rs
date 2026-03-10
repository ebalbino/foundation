#[path = "reflect/introspectable.rs"]
mod introspectable;
#[path = "reflect/registry.rs"]
mod registry;

use foundation::alloc::arena;
use foundation::reflect::registry as reflect_registry;
use foundation::reflect::{self, Description, Introspectable};

#[test]
fn introspect_returns_none_for_unknown_type_keys() {
    let arena = arena(16 * 1024);
    let registry = reflect_registry::initialize(arena.clone());
    let data = arena.allocate::<u32>(1).unwrap();

    assert!(reflect::introspect::<u32>(&registry, "missing", &data).is_none());
}

#[test]
fn introspect_uses_registered_description() {
    let arena = arena(16 * 1024);
    let mut registry = reflect_registry::initialize(arena.clone());
    let data = arena.allocate::<u32>(2).unwrap();

    registry
        .register("answer", <u32>::description())
        .unwrap_or(());

    let instance = reflect::introspect::<u32>(&registry, "answer", &data).unwrap();
    let debug = format!("{instance:?}");

    assert!(debug.contains("name: \"u32\""));
    assert!(debug.contains("size: 8"));
}

#[test]
fn description_values_are_cloneable() {
    let original = Description {
        name: "sample",
        size: 4,
        value: foundation::reflect::Value::Scalar {
            base: foundation::reflect::Base::Unsigned32,
        },
    };

    let clone = original.clone();

    assert_eq!(clone.name, "sample");
    assert_eq!(clone.size, 4);
}
