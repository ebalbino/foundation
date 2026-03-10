use foundation::alloc::arena;
use foundation::reflect::registry;
use foundation::reflect::{self, Description, Introspectable, Value};
use foundation::{registry_get, registry_set};

#[test]
fn initialize_registers_builtin_types() {
    let arena = arena(16 * 1024);
    let registry = registry::initialize(arena);

    let void_desc = registry.get("void").unwrap();
    let u32_desc = registry.get("u32").unwrap();
    let ptr_desc = registry.get("*const u8").unwrap();
    let option_desc = registry.get("Option<u32>").unwrap();
    let allocated_desc = registry.get("Allocated<u32>").unwrap();

    assert_eq!(void_desc.name, "void");
    assert_eq!(u32_desc.name, "u32");
    assert_eq!(ptr_desc.name, "*const");
    assert_eq!(option_desc.name, "option");
    assert_eq!(allocated_desc.name, "*arena");
}

#[test]
fn register_returns_none_on_insert_and_some_on_replace() {
    let arena = arena(16 * 1024);
    let mut registry = registry::initialize(arena);
    let first = Description {
        name: "custom",
        size: 1,
        value: Value::Scalar {
            base: foundation::reflect::Base::Unsigned8,
        },
    };
    let second = Description {
        name: "custom2",
        size: 2,
        value: Value::Scalar {
            base: foundation::reflect::Base::Unsigned16,
        },
    };

    assert!(registry.register("custom-key", first).is_none());
    assert!(registry.register("custom-key", second).is_some());
    assert_eq!(registry.get("custom-key").unwrap().name, "custom2");
}

#[test]
fn registry_macros_read_and_write_entries() {
    let arena = arena(16 * 1024);
    let mut registry = registry::initialize(arena);

    assert!(registry_set!(registry, u32, "alias-u32").is_none());

    let alias = registry_get!(registry, "alias-u32").unwrap();
    let builtin = registry_get!(registry, u32).unwrap();

    assert_eq!(alias.name, "u32");
    assert_eq!(builtin.name, "u32");
}

#[test]
fn introspect_uses_registered_aliases() {
    let arena = arena(16 * 1024);
    let mut registry = registry::initialize(arena.clone());
    let data = arena.allocate::<u16>(2).unwrap();

    registry_set!(registry, u16, "small-int");

    let instance = reflect::introspect::<u16>(&registry, "small-int", &data).unwrap();
    let debug = format!("{instance:?}");

    assert!(debug.contains("name: \"u16\""));
    assert!(debug.contains("size: 4"));
}

#[test]
fn registered_descriptions_are_shared_across_lookups() {
    let arena = arena(16 * 1024);
    let registry = registry::initialize(arena);

    let left = registry.get("u64").unwrap().clone();
    let right = registry.get("u64").unwrap().clone();

    assert!(std::rc::Rc::ptr_eq(&left, &right));
}
