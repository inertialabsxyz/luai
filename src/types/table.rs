use crate::{
    types::value::{LuaError, LuaString, LuaValue, MAX_TABLE_ENTRIES},
    vm::memory::Memory,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

//
// | Key Type  | Order                        |
// |-----------|------------------------------|
// | Integer   | Ascending numeric (`i64`)    |
// | String    | Ascending lexicographic (raw bytes) |
// | Boolean   | `false` < `true`             |

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum LuaKey {
    Integer(i64),
    String(LuaString), // interned or Arc<[u8]>
    Boolean(bool),
}

pub struct LuaTable {
    memory: Rc<RefCell<Memory>>,
    /// Integer keys 1..array_len stored at index key-1.
    array: Vec<LuaValue>,
    /// All other keys in canonical order.
    hash: BTreeMap<LuaKey, LuaValue>,
    /// Total logical entry count (array + hash), for memory accounting.
    entry_count: usize,
}

impl LuaTable {
    fn get(&self, key: &LuaKey) -> Option<&LuaValue> {
        if let LuaKey::Integer(i) = key {
            if *i >= 1 && *i <= self.array.len() as i64 {
                return Some(&self.array[(*i - 1) as usize]);
            }
        }
        self.hash.get(key)
    }

    fn rawset(&mut self, key: LuaKey, value: LuaValue) -> Result<(), LuaError> {
        if let LuaKey::Integer(i) = key {
            if i == i64::MIN {
                return Err(LuaError::ERR_RUNTIME);
            }
        }

        if matches!(value, LuaValue::Nil) {
            self.rawremove(&key);
            return Ok(());
        }

        match &key {
            LuaKey::Integer(i) if *i >= 1 => {
                let i = *i as usize;
                if i <= self.array.len() + 1 {
                    if i <= self.array.len() {
                        self.array[i - 1] = value;
                    } else {
                        self.array.push(value);
                        self.entry_count += 1;
                        loop {
                            let next = LuaKey::Integer((self.array.len() + 1) as i64);
                            if let Some(v) = self.hash.remove(&next) {
                                self.array.push(v);
                            } else {
                                break;
                            }
                        }
                    }
                } else {
                    let is_new = self.hash.insert(key, value).is_none();
                    if is_new {
                        self.entry_count += 1;
                    }
                }
            }
            _ => {
                let is_new = self.hash.insert(key, value).is_none();
                if is_new {
                    self.entry_count += 1;
                }
            }
        }

        if self.entry_count > MAX_TABLE_ENTRIES {
            return Err(LuaError::ERR_MEM);
        }

        self.memory.borrow_mut().track_alloc(self.charged_bytes())?;

        Ok(())
    }

    pub fn charged_bytes(&self) -> usize {
        let array_charge = self.array.capacity() * 16;
        let hash_capacity = next_power_of_two_capacity(self.hash.len()); // 0,4,8,16,...
        let hash_charge = 64 + hash_capacity * 40;
        array_charge + hash_charge
    }

    fn rawremove(&mut self, key: &LuaKey) {
        let removed = if let LuaKey::Integer(i) = key {
            if *i >= 1 && *i <= self.array.len() as i64 {
                self.array.remove((*i - 1) as usize);
                true
            } else {
                self.hash.remove(key).is_some()
            }
        } else {
            self.hash.remove(key).is_some()
        };

        if removed {
            self.entry_count -= 1;
        }
    }

    fn length(&self) -> i64 {
        self.array.len() as i64
    }

    fn next_sorted(&self, after: Option<&LuaKey>) -> Option<(LuaKey, &LuaValue)> {
        // Canonical order: integers 1..array_len, then hash keys (BTreeMap order).
        let array_len = self.array.len() as i64;

        // Determine where in the array portion to start.
        let array_start = match after {
            None => 1,
            Some(LuaKey::Integer(i)) if *i >= 1 && *i <= array_len => i + 1,
            _ => array_len + 1, // after is a hash key; skip array portion entirely
        };

        if array_start <= array_len {
            let idx = (array_start - 1) as usize;
            return Some((LuaKey::Integer(array_start), &self.array[idx]));
        }

        // Array portion exhausted; advance into hash portion.
        let hash_iter: Box<dyn Iterator<Item = (&LuaKey, &LuaValue)>> = match after {
            None | Some(LuaKey::Integer(_)) => Box::new(self.hash.iter()),
            Some(k) => Box::new(self.hash.range(k..).skip(1)),
        };

        hash_iter.into_iter().next().map(|(k, v)| (k.clone(), v))
    }
}

fn next_power_of_two_capacity(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let load_threshold = (n * 4 + 2) / 3; // ceil(n / 0.75)
    load_threshold.next_power_of_two().max(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::memory::Memory;
    use std::{cell::RefCell, rc::Rc, sync::Arc};

    fn make_table() -> LuaTable {
        LuaTable {
            memory: Rc::new(RefCell::new(Memory)),
            array: Vec::new(),
            hash: BTreeMap::new(),
            entry_count: 0,
        }
    }

    fn int(i: i64) -> LuaValue {
        LuaValue::Integer(i)
    }

    fn str_key(s: &str) -> LuaKey {
        LuaKey::String(Arc::from(s.as_bytes()))
    }

    fn as_int(v: &LuaValue) -> i64 {
        match v {
            LuaValue::Integer(i) => *i,
            _ => panic!("expected Integer"),
        }
    }

    // Array portion stores and retrieves t[1]..t[n]
    #[test]
    fn array_store_and_retrieve() {
        let mut t = make_table();
        t.rawset(LuaKey::Integer(1), int(10)).unwrap();
        t.rawset(LuaKey::Integer(2), int(20)).unwrap();
        t.rawset(LuaKey::Integer(3), int(30)).unwrap();

        assert_eq!(as_int(t.get(&LuaKey::Integer(1)).unwrap()), 10);
        assert_eq!(as_int(t.get(&LuaKey::Integer(2)).unwrap()), 20);
        assert_eq!(as_int(t.get(&LuaKey::Integer(3)).unwrap()), 30);
        assert_eq!(t.array.len(), 3);
    }

    // Hash portion stores string and boolean keys
    #[test]
    fn hash_stores_string_and_boolean_keys() {
        let mut t = make_table();
        t.rawset(str_key("hello"), int(1)).unwrap();
        t.rawset(LuaKey::Boolean(true), int(2)).unwrap();
        t.rawset(LuaKey::Boolean(false), int(3)).unwrap();

        assert_eq!(as_int(t.get(&str_key("hello")).unwrap()), 1);
        assert_eq!(as_int(t.get(&LuaKey::Boolean(true)).unwrap()), 2);
        assert_eq!(as_int(t.get(&LuaKey::Boolean(false)).unwrap()), 3);
        assert!(t.array.is_empty());
    }

    // Canonical iteration order: integers < strings < booleans
    #[test]
    fn canonical_iteration_order() {
        let mut t = make_table();
        t.rawset(LuaKey::Boolean(true), int(4)).unwrap();
        t.rawset(str_key("b"), int(3)).unwrap();
        t.rawset(LuaKey::Integer(1), int(1)).unwrap();
        t.rawset(LuaKey::Boolean(false), int(5)).unwrap();
        t.rawset(str_key("a"), int(2)).unwrap();

        let mut cursor = None;
        let mut keys = Vec::new();
        loop {
            match t.next_sorted(cursor.as_ref()) {
                None => break,
                Some((k, _)) => {
                    cursor = Some(k.clone());
                    keys.push(k);
                }
            }
        }

        assert_eq!(
            keys,
            vec![
                LuaKey::Integer(1),
                str_key("a"),
                str_key("b"),
                LuaKey::Boolean(false),
                LuaKey::Boolean(true),
            ]
        );
    }

    // Inserting integer key n+1 extends array portion
    #[test]
    fn array_grows_with_sequential_inserts() {
        let mut t = make_table();
        t.rawset(LuaKey::Integer(1), int(1)).unwrap();
        assert_eq!(t.array.len(), 1);
        t.rawset(LuaKey::Integer(2), int(2)).unwrap();
        assert_eq!(t.array.len(), 2);
        t.rawset(LuaKey::Integer(3), int(3)).unwrap();
        assert_eq!(t.array.len(), 3);
    }

    // Removing array tail key shrinks array portion
    #[test]
    fn array_shrinks_on_tail_remove() {
        let mut t = make_table();
        t.rawset(LuaKey::Integer(1), int(1)).unwrap();
        t.rawset(LuaKey::Integer(2), int(2)).unwrap();
        t.rawset(LuaKey::Integer(3), int(3)).unwrap();

        t.rawremove(&LuaKey::Integer(3));
        assert_eq!(t.array.len(), 2);
        assert!(t.get(&LuaKey::Integer(3)).is_none());
    }

    // Gap integer key goes to hash portion
    #[test]
    fn gap_integer_goes_to_hash() {
        let mut t = make_table();
        t.rawset(LuaKey::Integer(1), int(1)).unwrap();
        t.rawset(LuaKey::Integer(3), int(3)).unwrap(); // gap at 2

        assert_eq!(t.array.len(), 1);
        assert!(t.hash.contains_key(&LuaKey::Integer(3)));
        assert_eq!(as_int(t.get(&LuaKey::Integer(3)).unwrap()), 3);
    }

    // Setting a key to nil removes it
    #[test]
    fn rawset_nil_removes_key() {
        let mut t = make_table();
        t.rawset(LuaKey::Integer(1), int(42)).unwrap();
        assert!(t.get(&LuaKey::Integer(1)).is_some());

        t.rawset(LuaKey::Integer(1), LuaValue::Nil).unwrap();
        assert!(t.get(&LuaKey::Integer(1)).is_none());
        assert_eq!(t.entry_count, 0);
    }

    // length() returns contiguous array border
    #[test]
    fn length_returns_array_border() {
        let mut t = make_table();
        assert_eq!(t.length(), 0);
        t.rawset(LuaKey::Integer(1), int(1)).unwrap();
        t.rawset(LuaKey::Integer(2), int(2)).unwrap();
        assert_eq!(t.length(), 2);

        // String keys don't affect length
        t.rawset(str_key("x"), int(99)).unwrap();
        assert_eq!(t.length(), 2);
    }

    // Inserting hash key that fills a gap pulls it into array (consolidation)
    #[test]
    fn gap_fill_consolidates_into_array() {
        let mut t = make_table();
        t.rawset(LuaKey::Integer(1), int(1)).unwrap();
        t.rawset(LuaKey::Integer(3), int(3)).unwrap(); // 3 goes to hash (gap at 2)
        t.rawset(LuaKey::Integer(4), int(4)).unwrap(); // 4 goes to hash (gap at 2)

        assert_eq!(t.array.len(), 1);

        t.rawset(LuaKey::Integer(2), int(2)).unwrap(); // fills gap; should pull 3 and 4 in
        assert_eq!(t.array.len(), 4);
        assert!(!t.hash.contains_key(&LuaKey::Integer(3)));
        assert!(!t.hash.contains_key(&LuaKey::Integer(4)));
    }

    // Entry count exceeds max_table_entries → ERR_MEM
    #[test]
    fn entry_count_cap_returns_err_mem() {
        let mut t = make_table();
        // Fill to the limit
        for i in 1..=(MAX_TABLE_ENTRIES as i64) {
            t.rawset(LuaKey::Integer(i), int(i)).unwrap();
        }
        // One more should exceed the cap
        let result = t.rawset(LuaKey::Integer(MAX_TABLE_ENTRIES as i64 + 1), int(0));
        assert!(matches!(result, Err(LuaError::ERR_MEM)));
    }

    // charged_bytes() grows at power-of-2 thresholds
    #[test]
    fn charged_bytes_grows_at_power_of_two() {
        let mut t = make_table();
        let initial = t.charged_bytes();

        // Insert string keys to exercise the hash portion growth
        for i in 0..4u8 {
            t.rawset(str_key(&i.to_string()), int(i as i64)).unwrap();
        }
        let after_4 = t.charged_bytes();
        assert!(after_4 > initial);

        for i in 4..8u8 {
            t.rawset(str_key(&i.to_string()), int(i as i64)).unwrap();
        }
        let after_8 = t.charged_bytes();
        assert!(after_8 > after_4);
    }

    // Same insertion sequence → identical iteration order on two instances
    #[test]
    fn deterministic_iteration_across_instances() {
        let mut t1 = make_table();
        let mut t2 = make_table();

        let inserts: Vec<(LuaKey, LuaValue)> = vec![
            (str_key("z"), int(1)),
            (LuaKey::Integer(2), int(2)),
            (LuaKey::Boolean(false), int(3)),
            (str_key("a"), int(4)),
            (LuaKey::Integer(1), int(5)),
        ];

        for (k, v) in inserts {
            t1.rawset(k.clone(), v.clone()).unwrap();
            t2.rawset(k, v).unwrap();
        }

        let collect_keys = |t: &LuaTable| {
            let mut keys = Vec::new();
            let mut cursor = None;
            loop {
                match t.next_sorted(cursor.as_ref()) {
                    None => break,
                    Some((k, _)) => {
                        cursor = Some(k.clone());
                        keys.push(k);
                    }
                }
            }
            keys
        };

        assert_eq!(collect_keys(&t1), collect_keys(&t2));
    }
}
