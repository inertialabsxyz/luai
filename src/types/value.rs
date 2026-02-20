use std::sync::Arc;

use crate::types::table::LuaTable;
pub const MAX_TABLE_ENTRIES: usize = 50_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LuaString(Arc<[u8]>);

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
