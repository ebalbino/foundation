use crate::alloc::Arena;
use crate::alloc::string::{self, String};
use crate::rust_alloc::collections::BTreeMap;
use crate::rust_alloc::rc::Rc;

/// Interns strings into an arena-backed pool.
///
/// Repeated calls with the same value return clones of the same pooled
/// allocation.
#[derive(Debug, Clone)]
pub struct StringPool {
    arena: Rc<Arena>,
    map: BTreeMap<usize, String>,
}

/// Creates an empty [`StringPool`] backed by `arena`.
pub fn pool(arena: Rc<Arena>) -> StringPool {
    StringPool {
        arena: arena.clone(),
        map: BTreeMap::new(),
    }
}

impl StringPool {
    /// Returns the pooled string for `value`, inserting it if necessary.
    pub fn intern(&mut self, value: impl AsRef<str>) -> Option<String> {
        let value = value.as_ref();
        let key = fxhash::hash(value);

        if let Some(v) = self.map.get(&key) {
            return Some(v.clone());
        }

        let string = string::make(self.arena.clone(), value)?;
        self.map.insert(key, string.clone());
        Some(string)
    }

    /// Looks up a previously interned string by value.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&String> {
        let value = key.as_ref();
        let k = fxhash::hash(value);
        self.map.get(&k)
    }
}
