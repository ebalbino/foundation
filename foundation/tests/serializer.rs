use foundation::reflect::{Base, Description, Field, Introspectable, Value};
use foundation::serializer;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct Vec2 {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct Entity {
    id: u32,
    health: i32,
    pos: Vec2,
}

impl Introspectable for Vec2 {
    fn description() -> Description {
        Description {
            name: "Vec2",
            size: std::mem::size_of::<Vec2>(),
            value: Value::Composite {
                fields: vec![
                    Field {
                        desc: Description {
                            name: "f32",
                            size: std::mem::size_of::<f32>(),
                            value: Value::Scalar {
                                base: Base::Float32,
                            },
                        },
                        name: "x",
                        offset: std::mem::offset_of!(Vec2, x),
                    },
                    Field {
                        desc: Description {
                            name: "f32",
                            size: std::mem::size_of::<f32>(),
                            value: Value::Scalar {
                                base: Base::Float32,
                            },
                        },
                        name: "y",
                        offset: std::mem::offset_of!(Vec2, y),
                    },
                ],
            },
        }
    }
}

impl Introspectable for Entity {
    fn description() -> Description {
        Description {
            name: "Entity",
            size: std::mem::size_of::<Entity>(),
            value: Value::Composite {
                fields: vec![
                    Field {
                        desc: Description {
                            name: "u32",
                            size: std::mem::size_of::<u32>(),
                            value: Value::Scalar {
                                base: Base::Unsigned32,
                            },
                        },
                        name: "id",
                        offset: std::mem::offset_of!(Entity, id),
                    },
                    Field {
                        desc: Description {
                            name: "i32",
                            size: std::mem::size_of::<i32>(),
                            value: Value::Scalar {
                                base: Base::Signed32,
                            },
                        },
                        name: "health",
                        offset: std::mem::offset_of!(Entity, health),
                    },
                    Field {
                        desc: Vec2::description(),
                        name: "pos",
                        offset: std::mem::offset_of!(Entity, pos),
                    },
                ],
            },
        }
    }
}

#[test]
fn serialize_and_deserialize_scalar() {
    let value: i32 = -42;
    let encoded = serializer::serialize(&value).unwrap();
    let decoded: i32 = serializer::deserialize(&encoded).unwrap();

    assert_eq!(encoded, json::from(-42));
    assert_eq!(decoded, value);
}

#[test]
fn serialize_and_deserialize_composite() {
    let entity = Entity {
        id: 7,
        health: -3,
        pos: Vec2 { x: 10.5, y: -2.0 },
    };

    let encoded = serializer::serialize(&entity).unwrap();
    let decoded: Entity = serializer::deserialize(&encoded).unwrap();

    assert_eq!(encoded["id"], json::from(7));
    assert_eq!(encoded["health"], json::from(-3));
    assert_eq!(encoded["pos"]["x"], json::from(10.5));
    assert_eq!(encoded["pos"]["y"], json::from(-2.0));
    assert_eq!(decoded, entity);
}

#[test]
fn deserialize_missing_field_fails() {
    let source = json::parse(r#"{ "id": 1, "pos": { "x": 0.0, "y": 1.0 } }"#).unwrap();
    let result = serializer::deserialize::<Entity>(&source);

    assert!(matches!(
        result,
        Err(serializer::Error::MissingField("health"))
    ));
}

#[test]
fn deserialize_out_of_range_fails() {
    let source =
        json::parse(r#"{ "id": 999999999999, "health": 1, "pos": { "x": 0.0, "y": 0.0 } }"#)
            .unwrap();
    let result = serializer::deserialize::<Entity>(&source);

    assert!(matches!(
        result,
        Err(serializer::Error::OutOfRange {
            expected: "u32",
            ..
        })
    ));
}
