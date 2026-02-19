use std::sync::Arc;
pub const MAX_TABLE_ENTRIES: usize = 50_000;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Integer(i64),
}
#[derive(Debug)]
pub enum LuaError {
    ERR_RUNTIME,
    ERR_MEM,
}
pub type LuaString = Arc<[u8]>;
