use std::sync::Arc;

use crate::types::table::LuaTable;
pub const MAX_TABLE_ENTRIES: usize = 50_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LuaString(Arc<[u8]>);

impl LuaString {
    fn from_bytes(bytes: &[u8]) -> Self {
        LuaString(Arc::from(bytes))
    }

    fn from_str(s: &str) -> Self {
        LuaString(Arc::from(s.as_bytes()))
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct LuaFunction;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Integer(i64),
    String(LuaString),
    Table(LuaTable),
    Function(LuaFunction),
}
#[derive(Debug)]
pub enum LuaError {
    ERR_RUNTIME,
    ERR_MEM,
}
