use crate::types::value::LuaError;
pub struct Memory;
impl Memory {
    pub fn track_alloc(&mut self, _new_bytes: usize) -> Result<(), LuaError> {
        Ok(())
    }
}
