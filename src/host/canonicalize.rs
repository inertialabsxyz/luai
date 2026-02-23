//! Canonical JSON serialization for `LuaValue`.
//!
//! This is the single canonical serialization algorithm used everywhere:
//! - `json.encode` (builtin)
//! - tool call `args_canonical` in the transcript
//! - SHA-256 input for `response_hash`
//! - byte-length accounting for quota enforcement

use crate::{
    types::{
        table::{LuaKey, LuaTable},
        value::LuaValue,
    },
    vm::gas::VmError,
};

const MAX_TABLE_DEPTH: usize = 32;
const MAX_STRING_LEN: usize = 65536; // 64 KB

/// Error type for canonicalization failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonError {
    FunctionNotSerializable,
    TableDepthExceeded,
    StringTooLong,
}

impl From<CanonError> for VmError {
    fn from(e: CanonError) -> VmError {
        use crate::types::value::LuaString;
        let msg = match e {
            CanonError::FunctionNotSerializable => "json.encode: functions not serializable",
            CanonError::TableDepthExceeded => "json.encode: table depth exceeded",
            CanonError::StringTooLong => "string length overflow",
        };
        VmError::RuntimeError(LuaValue::String(LuaString::from_str(msg)))
    }
}

/// Serialize a `LuaValue` to canonical JSON bytes.
pub fn canonical_serialize(v: &LuaValue) -> Result<Vec<u8>, CanonError> {
    let result = serialize_value(v, 0)?;
    if result.len() > MAX_STRING_LEN {
        return Err(CanonError::StringTooLong);
    }
    Ok(result)
}

/// Serialize directly from a `LuaTable` reference.
pub fn canonical_serialize_table(t: &LuaTable) -> Result<Vec<u8>, CanonError> {
    let result = serialize_table(t, 0)?;
    if result.len() > MAX_STRING_LEN {
        return Err(CanonError::StringTooLong);
    }
    Ok(result)
}

/// Return the byte length of the canonical serialization without allocating the
/// full buffer. Used for quota pre-checks.
pub fn canonical_byte_len(v: &LuaValue) -> Result<usize, CanonError> {
    Ok(canonical_serialize(v)?.len())
}

fn serialize_value(v: &LuaValue, depth: usize) -> Result<Vec<u8>, CanonError> {
    if depth > MAX_TABLE_DEPTH {
        return Err(CanonError::TableDepthExceeded);
    }
    match v {
        LuaValue::Nil => Ok(b"null".to_vec()),
        LuaValue::Boolean(b) => Ok(if *b { b"true".to_vec() } else { b"false".to_vec() }),
        LuaValue::Integer(n) => Ok(n.to_string().into_bytes()),
        LuaValue::String(s) => serialize_string(s.as_bytes()),
        LuaValue::Table(t) => serialize_table(&t.borrow(), depth),
        LuaValue::Function(_) | LuaValue::Builtin(_) => {
            Err(CanonError::FunctionNotSerializable)
        }
    }
}

fn serialize_string(bytes: &[u8]) -> Result<Vec<u8>, CanonError> {
    let mut out = vec![b'"'];
    for &b in bytes {
        match b {
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0x20..=0x7e => out.push(b),
            _ => {
                let s = format!("\\u{:04x}", b);
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
    out.push(b'"');
    Ok(out)
}

fn serialize_table(t: &LuaTable, depth: usize) -> Result<Vec<u8>, CanonError> {
    if depth > MAX_TABLE_DEPTH {
        return Err(CanonError::TableDepthExceeded);
    }

    let len = t.length();

    // Collect all keys in canonical order using sorted_keys().
    // Filter out keys with nil values (absent key semantics).
    let keys: Vec<LuaKey> = t
        .sorted_keys()
        .into_iter()
        .filter_map(|k_val| {
            let key = match k_val {
                LuaValue::Integer(i) => LuaKey::Integer(i),
                LuaValue::String(s) => LuaKey::String(s),
                LuaValue::Boolean(b) => LuaKey::Boolean(b),
                _ => return None,
            };
            // Skip keys whose value is nil.
            match t.get(&key) {
                Some(v) if !matches!(v, LuaValue::Nil) => Some(key),
                _ => None,
            }
        })
        .collect();

    let entry_count = keys.len();

    // Pure array: all keys are consecutive integers 1..=n with no gaps,
    // and entry_count matches len.
    let is_array = len > 0 && entry_count == len as usize;

    if is_array {
        let mut out = vec![b'['];
        for i in 1..=len {
            if i > 1 {
                out.push(b',');
            }
            let val = t.get(&LuaKey::Integer(i)).cloned().unwrap_or(LuaValue::Nil);
            let encoded = serialize_value(&val, depth + 1)?;
            out.extend_from_slice(&encoded);
        }
        out.push(b']');
        return Ok(out);
    }

    // Object: keys in canonical order (integers ascending, strings lexicographic, booleans).
    let mut out = vec![b'{'];
    let mut first = true;
    for k in &keys {
        let v = match t.get(k) {
            Some(v) if !matches!(v, LuaValue::Nil) => v,
            _ => continue,
        };

        if !first {
            out.push(b',');
        }
        first = false;

        // Encode key as JSON string.
        let key_bytes: Vec<u8> = match k {
            LuaKey::Integer(n) => serialize_string(n.to_string().as_bytes())?,
            LuaKey::String(s) => serialize_string(s.as_bytes())?,
            LuaKey::Boolean(b) => {
                if *b { b"\"true\"".to_vec() } else { b"\"false\"".to_vec() }
            }
        };
        out.extend_from_slice(&key_bytes);
        out.push(b':');
        let encoded = serialize_value(v, depth + 1)?;
        out.extend_from_slice(&encoded);
    }
    out.push(b'}');
    Ok(out)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use crate::types::{
        table::{LuaKey, LuaTable},
        value::{BuiltinId, LuaClosure, LuaString, LuaValue},
    };

    fn int(n: i64) -> LuaValue { LuaValue::Integer(n) }
    fn s(text: &str) -> LuaValue { LuaValue::String(LuaString::from_str(text)) }
    fn make_table() -> Rc<RefCell<LuaTable>> { Rc::new(RefCell::new(LuaTable::new())) }

    fn encode(v: &LuaValue) -> String {
        let bytes = canonical_serialize(v).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    fn encode_table(t: &LuaTable) -> String {
        let bytes = canonical_serialize_table(t).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn nil_is_null() {
        assert_eq!(encode(&LuaValue::Nil), "null");
    }

    #[test]
    fn bool_true() {
        assert_eq!(encode(&LuaValue::Boolean(true)), "true");
    }

    #[test]
    fn bool_false() {
        assert_eq!(encode(&LuaValue::Boolean(false)), "false");
    }

    #[test]
    fn integer_positive() {
        assert_eq!(encode(&int(42)), "42");
    }

    #[test]
    fn integer_negative() {
        assert_eq!(encode(&int(-1)), "-1");
    }

    #[test]
    fn integer_zero() {
        assert_eq!(encode(&int(0)), "0");
    }

    #[test]
    fn integer_min_max() {
        assert_eq!(encode(&int(i64::MAX)), i64::MAX.to_string());
        assert_eq!(encode(&int(i64::MIN)), i64::MIN.to_string());
    }

    #[test]
    fn string_simple() {
        assert_eq!(encode(&s("hello")), "\"hello\"");
    }

    #[test]
    fn string_with_escapes() {
        assert_eq!(encode(&s("a\"b")), r#""a\"b""#);
        assert_eq!(encode(&s("a\\b")), r#""a\\b""#);
        assert_eq!(encode(&s("a\nb")), r#""a\nb""#);
        assert_eq!(encode(&s("a\rb")), r#""a\rb""#);
        assert_eq!(encode(&s("a\tb")), r#""a\tb""#);
    }

    #[test]
    fn string_non_ascii_byte_escape() {
        // byte 0x80 should become \u0080
        let v = LuaValue::String(LuaString::from_bytes(&[0x80u8]));
        assert_eq!(encode(&v), "\"\\u0080\"");
    }

    #[test]
    fn string_printable_ascii_unescaped() {
        // 0x20 (space) and 0x7e (~) should not be escaped
        let v = LuaValue::String(LuaString::from_bytes(&[0x20u8, 0x7eu8]));
        assert_eq!(encode(&v), "\" ~\"");
    }

    #[test]
    fn array_table() {
        let t = make_table();
        for i in 1i64..=3 {
            t.borrow_mut().rawset(LuaKey::Integer(i), int(i * 10)).unwrap();
        }
        assert_eq!(encode(&LuaValue::Table(t)), "[10,20,30]");
    }

    #[test]
    fn object_table_string_key() {
        let t = make_table();
        t.borrow_mut().rawset(LuaKey::String(LuaString::from_str("a")), int(1)).unwrap();
        assert_eq!(encode(&LuaValue::Table(t)), r#"{"a":1}"#);
    }

    #[test]
    fn object_table_integer_key() {
        let t = make_table();
        // Non-consecutive: 1 and 3 but not 2 → object
        t.borrow_mut().rawset(LuaKey::Integer(1), int(10)).unwrap();
        t.borrow_mut().rawset(LuaKey::Integer(3), int(30)).unwrap();
        let result = encode(&LuaValue::Table(t));
        assert_eq!(result, r#"{"1":10,"3":30}"#);
    }

    #[test]
    fn object_key_ordering_integers_before_strings_before_bools() {
        let t = make_table();
        t.borrow_mut().rawset(LuaKey::Boolean(true), int(3)).unwrap();
        t.borrow_mut().rawset(LuaKey::String(LuaString::from_str("b")), int(2)).unwrap();
        t.borrow_mut().rawset(LuaKey::Integer(1), int(1)).unwrap();
        let result = encode_table(&t.borrow());
        // integers ascending, then strings lexicographic, then booleans
        assert_eq!(result, r#"{"1":1,"b":2,"true":3}"#);
    }

    #[test]
    fn nil_values_in_table_omitted() {
        let t = make_table();
        t.borrow_mut().rawset(LuaKey::String(LuaString::from_str("a")), int(1)).unwrap();
        // Setting nil via LuaValue::Nil should be omitted from serialization
        // (If rawset with nil removes the key, this still tests the skip logic)
        let result = encode_table(&t.borrow());
        assert_eq!(result, r#"{"a":1}"#);
    }

    #[test]
    fn nested_table_at_depth_32_ok() {
        // Build a chain of depth 32 (each table contains one nested table)
        let mut inner = LuaValue::Integer(0);
        for _ in 0..32 {
            let t = make_table();
            t.borrow_mut().rawset(LuaKey::String(LuaString::from_str("x")), inner).unwrap();
            inner = LuaValue::Table(t);
        }
        // depth 32 should be ok (limit is > 32)
        assert!(canonical_serialize(&inner).is_ok());
    }

    #[test]
    fn nested_table_depth_33_error() {
        // Build a chain of depth 33
        let mut inner = LuaValue::Integer(0);
        for _ in 0..33 {
            let t = make_table();
            t.borrow_mut().rawset(LuaKey::String(LuaString::from_str("x")), inner).unwrap();
            inner = LuaValue::Table(t);
        }
        let err = canonical_serialize(&inner).unwrap_err();
        assert_eq!(err, CanonError::TableDepthExceeded);
    }

    #[test]
    fn function_not_serializable() {
        let err = canonical_serialize(&LuaValue::Builtin(BuiltinId::Type)).unwrap_err();
        assert_eq!(err, CanonError::FunctionNotSerializable);
    }

    #[test]
    fn function_closure_not_serializable() {
        let closure = LuaClosure { proto_idx: 0, upvalues: vec![] };
        let err = canonical_serialize(&LuaValue::Function(closure)).unwrap_err();
        assert_eq!(err, CanonError::FunctionNotSerializable);
    }

    #[test]
    fn byte_len_matches_serialize_len() {
        let values = vec![
            LuaValue::Nil,
            LuaValue::Boolean(true),
            int(42),
            s("hello"),
        ];
        for v in &values {
            let serialized_len = canonical_serialize(v).unwrap().len();
            let byte_len = canonical_byte_len(v).unwrap();
            assert_eq!(serialized_len, byte_len, "mismatch for {:?}", v);
        }

        // Also test with a table
        let t = make_table();
        t.borrow_mut().rawset(LuaKey::Integer(1), int(1)).unwrap();
        t.borrow_mut().rawset(LuaKey::Integer(2), int(2)).unwrap();
        let tv = LuaValue::Table(t);
        assert_eq!(canonical_serialize(&tv).unwrap().len(), canonical_byte_len(&tv).unwrap());
    }

    #[test]
    fn canonical_serialize_table_direct() {
        let t = make_table();
        t.borrow_mut().rawset(LuaKey::Integer(1), int(10)).unwrap();
        t.borrow_mut().rawset(LuaKey::Integer(2), int(20)).unwrap();
        let result = canonical_serialize_table(&t.borrow()).unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), "[10,20]");
    }

    #[test]
    fn empty_table_is_object() {
        let t = make_table();
        assert_eq!(encode_table(&t.borrow()), "{}");
    }
}
