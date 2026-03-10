use foundation::alloc::Allocated;
use foundation::reflect::{Base, Description, Field, Introspectable, Value};
use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
use std::rc::Rc;

struct Pair {
    left: u16,
    right: u8,
}

impl Introspectable for Pair {
    fn description() -> Description {
        Description {
            name: "Pair",
            size: std::mem::size_of::<Self>(),
            value: Value::Composite {
                fields: vec![
                    Field {
                        desc: <u16>::description(),
                        name: "left",
                        offset: std::mem::offset_of!(Pair, left),
                    },
                    Field {
                        desc: <u8>::description(),
                        name: "right",
                        offset: std::mem::offset_of!(Pair, right),
                    },
                ],
            },
        }
    }
}

#[test]
fn scalar_types_report_expected_base_and_size() {
    let desc = <u32>::description();

    assert_eq!(desc.name, "u32");
    assert_eq!(desc.size, 4);
    match desc.value {
        Value::Scalar { base } => assert!(matches!(base, Base::Unsigned32)),
        other => panic!("expected scalar description, got {other:?}"),
    }
}

#[test]
fn pointer_like_types_describe_their_pointee() {
    let raw_desc = <*const u16>::description();
    let nonnull_desc = <NonNull<u16>>::description();
    let arena_desc = <Allocated<u16>>::description();

    for desc in [raw_desc, nonnull_desc, arena_desc] {
        match desc.value {
            Value::Reference { pointee } => {
                assert_eq!(pointee.name, "u16");
                assert_eq!(pointee.size, 2);
            }
            other => panic!("expected reference description, got {other:?}"),
        }
    }
}

#[test]
fn option_and_result_descriptions_are_enumerations() {
    let option_desc = <Option<u8>>::description();
    let result_desc = <Result<u16, i8>>::description();

    match option_desc.value {
        Value::Enumeration { values } => {
            assert_eq!(option_desc.name, "option");
            assert_eq!(values.len(), 2);
            assert_eq!(values[0].name, "void");
            assert_eq!(values[1].name, "u8");
        }
        other => panic!("expected option enumeration, got {other:?}"),
    }

    match result_desc.value {
        Value::Enumeration { values } => {
            assert_eq!(result_desc.name, "result");
            assert_eq!(values.len(), 2);
            assert_eq!(values[0].name, "i8");
            assert_eq!(values[1].name, "u16");
        }
        other => panic!("expected result enumeration, got {other:?}"),
    }
}

#[test]
fn composite_descriptions_expose_field_names_and_offsets() {
    let desc = Pair::description();

    assert_eq!(desc.name, "Pair");
    assert_eq!(desc.size, std::mem::size_of::<Pair>());
    match desc.value {
        Value::Composite { fields } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "left");
            assert_eq!(fields[0].desc.name, "u16");
            assert_eq!(fields[0].offset, std::mem::offset_of!(Pair, left));
            assert_eq!(fields[1].name, "right");
            assert_eq!(fields[1].desc.name, "u8");
            assert_eq!(fields[1].offset, std::mem::offset_of!(Pair, right));
        }
        other => panic!("expected composite description, got {other:?}"),
    }
}

#[test]
fn wrappers_preserve_inner_descriptions() {
    let cell_desc = <Cell<u32>>::description();
    let refcell_desc = <RefCell<u32>>::description();
    let rc_desc = <Rc<u32>>::description();
    let vec_desc = <Vec<u32>>::description();

    for desc in [cell_desc, refcell_desc, rc_desc, vec_desc] {
        match desc.value {
            Value::Reference { pointee } => assert_eq!(pointee.name, "u32"),
            other => panic!("expected wrapper reference, got {other:?}"),
        }
    }
}
