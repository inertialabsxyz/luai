use luai::{
    HostInterface,
    types::{
        table::{LuaKey, LuaTable},
        value::{LuaString, LuaValue},
    },
};

pub struct ProverHost;

impl HostInterface for ProverHost {
    fn call_tool(&mut self, name: &str, _args: &LuaTable) -> Result<LuaTable, String> {
        let mut resp = LuaTable::new();
        let str_key = |s: &str| LuaKey::String(LuaString::from_str(s));
        match name {
            // random: returns a random integer, currently 42
            "random" => {
                resp.rawset(str_key("result"), LuaValue::Integer(42))
                    .unwrap();
            }
            // fail: always errors
            "fail" => return Err("this tool always fails".into()),
            other => return Err(format!("unknown tool '{other}'")),
        }
        Ok(resp)
    }
}
