use crate::alloc::{Allocated, Arena, StringPool, StringRef, string_pool};
use crate::reflect::Description;
use crate::reflect::Introspectable;
use crate::rust_alloc::collections::BTreeMap;
use crate::rust_alloc::rc::Rc;

/// Name-based storage for reflected type descriptions.
///
/// Keys are interned into a [`StringPool`] so registry lookups can reuse stable
/// arena-backed string storage.
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    pool: StringPool,
    map: BTreeMap<StringRef, Rc<Description>>,
}

impl TypeRegistry {
    /// Registers `description` under `key`.
    ///
    /// Returns `Some(())` only when an existing registration was replaced. A new
    /// insertion returns `None`.
    pub fn register(&mut self, key: impl AsRef<str>, description: Description) -> Option<()> {
        let key = self.pool.intern(key)?;

        if let Some(_) = self.map.insert(key.borrow(), Rc::new(description)) {
            return Some(());
        }

        None
    }

    /// Retrieves a previously registered description by key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Rc<Description>> {
        let key = self.pool.get(key)?;
        self.map.get(&key.borrow())
    }
}

/// Registers a type description in a [`TypeRegistry`].
///
/// `registry_set!(registry, MyType)` uses `stringify!(MyType)` as the key.
/// `registry_set!(registry, MyType, "name")` uses the explicit name instead.
#[macro_export]
macro_rules! registry_set {
    ($r:ident, $t:ty) => {
        $r.register(stringify!($t), <$t>::description());
    };
    ($r:ident, $t:ty, $name:expr) => {
        $r.register($name, <$t>::description());
    };
}

/// Retrieves a registered type description from a [`TypeRegistry`].
#[macro_export]
macro_rules! registry_get {
    ($r:ident, $t:ty) => {
        $r.get(stringify!($t))
    };
    ($r:ident, $name:expr) => {
        $r.get($name)
    };
}

/// Creates a registry pre-populated with the built-in reflected primitives and
/// common wrappers supported by this crate.
pub fn initialize(arena: Rc<Arena>) -> TypeRegistry {
    let pool = string_pool(arena.clone());
    let mut registry = TypeRegistry {
        pool,
        map: BTreeMap::new(),
    };

    registry_set!(registry, (), "void");
    registry_set!(registry, u8);
    registry_set!(registry, u16);
    registry_set!(registry, u32);
    registry_set!(registry, u64);
    registry_set!(registry, usize);
    registry_set!(registry, i8);
    registry_set!(registry, i16);
    registry_set!(registry, i32);
    registry_set!(registry, i64);
    registry_set!(registry, isize);
    registry_set!(registry, f32);
    registry_set!(registry, f64);
    registry_set!(registry, *const (), "*const void");
    registry_set!(registry, *const u8);
    registry_set!(registry, *const u16);
    registry_set!(registry, *const u32);
    registry_set!(registry, *const u64);
    registry_set!(registry, *const usize);
    registry_set!(registry, *const i8);
    registry_set!(registry, *const i16);
    registry_set!(registry, *const i32);
    registry_set!(registry, *const i64);
    registry_set!(registry, *const isize);
    registry_set!(registry, *const f32);
    registry_set!(registry, *const f64);
    registry_set!(registry, *mut (), "*mut void");
    registry_set!(registry, *mut u8);
    registry_set!(registry, *mut u16);
    registry_set!(registry, *mut u32);
    registry_set!(registry, *mut u64);
    registry_set!(registry, *mut usize);
    registry_set!(registry, *mut i8);
    registry_set!(registry, *mut i16);
    registry_set!(registry, *mut i32);
    registry_set!(registry, *mut i64);
    registry_set!(registry, *mut isize);
    registry_set!(registry, *mut f32);
    registry_set!(registry, *mut f64);
    registry_set!(registry, Option<()>);
    registry_set!(registry, Option<u8>);
    registry_set!(registry, Option<u16>);
    registry_set!(registry, Option<u32>);
    registry_set!(registry, Option<u64>);
    registry_set!(registry, Option<usize>);
    registry_set!(registry, Option<i8>);
    registry_set!(registry, Option<i16>);
    registry_set!(registry, Option<i32>);
    registry_set!(registry, Option<i64>);
    registry_set!(registry, Option<isize>);
    registry_set!(registry, Option<f32>);
    registry_set!(registry, Option<f64>);
    registry_set!(registry, Allocated<()>);
    registry_set!(registry, Allocated<u8>);
    registry_set!(registry, Allocated<u16>);
    registry_set!(registry, Allocated<u32>);
    registry_set!(registry, Allocated<u64>);
    registry_set!(registry, Allocated<usize>);
    registry_set!(registry, Allocated<i8>);
    registry_set!(registry, Allocated<i16>);
    registry_set!(registry, Allocated<i32>);
    registry_set!(registry, Allocated<i64>);
    registry_set!(registry, Allocated<isize>);
    registry_set!(registry, Allocated<f32>);
    registry_set!(registry, Allocated<f64>);

    registry
}
