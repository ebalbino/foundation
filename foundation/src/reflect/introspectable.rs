use crate::alloc::Allocated;
use crate::reflect::{Base, Description, Value};
use crate::rust_alloc::boxed::Box;
use crate::rust_alloc::rc::Rc;
use crate::rust_alloc::vec;
use crate::rust_alloc::vec::Vec;
use core::cell::{Cell, RefCell};
use core::ptr::NonNull;

/// Produces a structural description for a type.
///
/// Implementations should describe the layout category of the type rather than its
/// runtime value. Most user-defined structs are expected to implement this through
/// `#[derive(Reflect)]` from `foundation_derive`.
pub trait Introspectable {
    /// Returns the reflected description for `Self`.
    fn description() -> Description;
}

macro_rules! introspect_scalar {
    ($t:ty, $name:literal, $size:expr, $base:expr) => {
        impl Introspectable for $t {
            fn description() -> Description {
                Description {
                    name: $name,
                    size: $size,
                    value: Value::Scalar { base: $base },
                }
            }
        }
    };
}

introspect_scalar!((), "void", ::core::mem::size_of::<()>(), Base::Void);
introspect_scalar!(u8, "u8", 1, Base::Unsigned8);
introspect_scalar!(u16, "u16", 2, Base::Unsigned16);
introspect_scalar!(u32, "u32", 4, Base::Unsigned32);
introspect_scalar!(u64, "u64", 8, Base::Unsigned64);
introspect_scalar!(
    usize,
    "usize",
    ::core::mem::size_of::<usize>(),
    Base::UnsignedPtr
);
introspect_scalar!(i8, "i8", 1, Base::Signed8);
introspect_scalar!(i16, "i16", 2, Base::Signed16);
introspect_scalar!(i32, "i32", 4, Base::Signed32);
introspect_scalar!(i64, "i64", 8, Base::Signed64);
introspect_scalar!(
    isize,
    "isize",
    ::core::mem::size_of::<isize>(),
    Base::SignedPtr
);
introspect_scalar!(f32, "f32", 4, Base::Float32);
introspect_scalar!(f64, "f64", 8, Base::Float64);

impl<T: Introspectable> Introspectable for *const T {
    fn description() -> Description {
        Description {
            name: "*const",
            size: ::core::mem::size_of::<*const T>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for *mut T {
    fn description() -> Description {
        Description {
            name: "*mut",
            size: ::core::mem::size_of::<*const T>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for NonNull<T> {
    fn description() -> Description {
        Description {
            name: "*nonnull",
            size: ::core::mem::size_of::<NonNull<T>>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for Box<T> {
    fn description() -> Description {
        Description {
            name: "*unique",
            size: ::core::mem::size_of::<T>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for Rc<T> {
    fn description() -> Description {
        Description {
            name: "*shared",
            size: ::core::mem::size_of::<T>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for Option<T> {
    fn description() -> Description {
        Description {
            name: "option",
            size: ::core::mem::size_of::<Option<T>>(),
            value: Value::Enumeration {
                values: vec![<()>::description(), T::description()],
            },
        }
    }
}

impl<T: Introspectable, E: Introspectable> Introspectable for Result<T, E> {
    fn description() -> Description {
        Description {
            name: "result",
            size: ::core::mem::size_of::<Result<T, E>>(),
            value: Value::Enumeration {
                values: vec![E::description(), T::description()],
            },
        }
    }
}

impl<T: Introspectable> Introspectable for Cell<T> {
    fn description() -> Description {
        Description {
            name: "cell",
            size: ::core::mem::size_of::<Cell<T>>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for RefCell<T> {
    fn description() -> Description {
        Description {
            name: "refcell",
            size: ::core::mem::size_of::<RefCell<T>>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for Vec<T> {
    fn description() -> Description {
        Description {
            name: "vec",
            size: ::core::mem::size_of::<Vec<T>>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}

impl<T: Introspectable> Introspectable for Allocated<T> {
    fn description() -> Description {
        Description {
            name: "*arena",
            size: ::core::mem::size_of::<Allocated<T>>(),
            value: Value::Reference {
                pointee: Box::new(T::description()),
            },
        }
    }
}
