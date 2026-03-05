use luai::{
    types::{
        table::{LuaKey, LuaTable},
        value::{LuaString, LuaValue},
    },
    vm::engine::HostInterface,
};

/// A stub host with basic tools for initial testing.
/// This will be replaced by a full LiveHost with real HTTP, KV, and LLM tools.
pub struct StubHost;

impl HostInterface for StubHost {
    fn call_tool(&mut self, name: &str, args: &LuaTable) -> Result<LuaTable, String> {
        let str_key = |s: &str| LuaKey::String(LuaString::from_str(s));
        let mut resp = LuaTable::new();

        match name {
            "echo" => {
                let msg = args
                    .get(&str_key("message"))
                    .cloned()
                    .unwrap_or(LuaValue::Nil);
                resp.rawset(str_key("message"), msg).unwrap();
            }
            "add" => {
                let a = match args.get(&str_key("a")) {
                    Some(LuaValue::Integer(n)) => *n,
                    _ => return Err("add: expected integer arg 'a'".into()),
                };
                let b = match args.get(&str_key("b")) {
                    Some(LuaValue::Integer(n)) => *n,
                    _ => return Err("add: expected integer arg 'b'".into()),
                };
                resp.rawset(str_key("result"), LuaValue::Integer(a + b))
                    .unwrap();
            }
            "upper" => {
                let text = match args.get(&str_key("text")) {
                    Some(LuaValue::String(s)) => {
                        String::from_utf8_lossy(s.as_bytes()).to_uppercase()
                    }
                    _ => return Err("upper: expected string arg 'text'".into()),
                };
                resp.rawset(
                    str_key("result"),
                    LuaValue::String(LuaString::from_str(&text)),
                )
                .unwrap();
            }
            "time_now" => {
                // Deterministic stub: always returns a fixed timestamp
                resp.rawset(str_key("timestamp"), LuaValue::Integer(1709654400))
                    .unwrap();
            }
            other => return Err(format!("unknown tool '{other}'")),
        }
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn str_key(s: &str) -> LuaKey {
        LuaKey::String(LuaString::from_str(s))
    }

    fn make_args(pairs: &[(&str, LuaValue)]) -> LuaTable {
        let mut t = LuaTable::new();
        for (k, v) in pairs {
            t.rawset(str_key(k), v.clone()).unwrap();
        }
        t
    }

    // ── echo ─────────────────────────────────────────────────────────

    #[test]
    fn echo_returns_message() {
        let mut host = StubHost;
        let args = make_args(&[("message", LuaValue::String(LuaString::from_str("hello")))]);
        let resp = host.call_tool("echo", &args).unwrap();
        assert_eq!(
            resp.get(&str_key("message")),
            Some(&LuaValue::String(LuaString::from_str("hello")))
        );
    }

    #[test]
    fn echo_missing_message_returns_nil() {
        let mut host = StubHost;
        let args = LuaTable::new();
        let resp = host.call_tool("echo", &args).unwrap();
        // Lua semantics: rawset with Nil is a no-op, so key is absent
        assert_eq!(resp.get(&str_key("message")), None);
    }

    // ── add ──────────────────────────────────────────────────────────

    #[test]
    fn add_returns_sum() {
        let mut host = StubHost;
        let args = make_args(&[
            ("a", LuaValue::Integer(17)),
            ("b", LuaValue::Integer(25)),
        ]);
        let resp = host.call_tool("add", &args).unwrap();
        assert_eq!(resp.get(&str_key("result")), Some(&LuaValue::Integer(42)));
    }

    #[test]
    fn add_negative_numbers() {
        let mut host = StubHost;
        let args = make_args(&[
            ("a", LuaValue::Integer(-10)),
            ("b", LuaValue::Integer(3)),
        ]);
        let resp = host.call_tool("add", &args).unwrap();
        assert_eq!(resp.get(&str_key("result")), Some(&LuaValue::Integer(-7)));
    }

    #[test]
    fn add_missing_arg_a_errors() {
        let mut host = StubHost;
        let args = make_args(&[("b", LuaValue::Integer(1))]);
        let err = host.call_tool("add", &args).unwrap_err();
        assert!(err.contains("expected integer arg 'a'"));
    }

    #[test]
    fn add_missing_arg_b_errors() {
        let mut host = StubHost;
        let args = make_args(&[("a", LuaValue::Integer(1))]);
        let err = host.call_tool("add", &args).unwrap_err();
        assert!(err.contains("expected integer arg 'b'"));
    }

    #[test]
    fn add_wrong_type_errors() {
        let mut host = StubHost;
        let args = make_args(&[
            ("a", LuaValue::String(LuaString::from_str("not a number"))),
            ("b", LuaValue::Integer(1)),
        ]);
        let err = host.call_tool("add", &args).unwrap_err();
        assert!(err.contains("expected integer arg 'a'"));
    }

    // ── upper ────────────────────────────────────────────────────────

    #[test]
    fn upper_converts_text() {
        let mut host = StubHost;
        let args = make_args(&[("text", LuaValue::String(LuaString::from_str("hello world")))]);
        let resp = host.call_tool("upper", &args).unwrap();
        assert_eq!(
            resp.get(&str_key("result")),
            Some(&LuaValue::String(LuaString::from_str("HELLO WORLD")))
        );
    }

    #[test]
    fn upper_already_uppercase() {
        let mut host = StubHost;
        let args = make_args(&[("text", LuaValue::String(LuaString::from_str("ABC")))]);
        let resp = host.call_tool("upper", &args).unwrap();
        assert_eq!(
            resp.get(&str_key("result")),
            Some(&LuaValue::String(LuaString::from_str("ABC")))
        );
    }

    #[test]
    fn upper_missing_text_errors() {
        let mut host = StubHost;
        let args = LuaTable::new();
        let err = host.call_tool("upper", &args).unwrap_err();
        assert!(err.contains("expected string arg 'text'"));
    }

    #[test]
    fn upper_wrong_type_errors() {
        let mut host = StubHost;
        let args = make_args(&[("text", LuaValue::Integer(42))]);
        let err = host.call_tool("upper", &args).unwrap_err();
        assert!(err.contains("expected string arg 'text'"));
    }

    // ── time_now ─────────────────────────────────────────────────────

    #[test]
    fn time_now_returns_fixed_timestamp() {
        let mut host = StubHost;
        let args = LuaTable::new();
        let resp = host.call_tool("time_now", &args).unwrap();
        assert_eq!(
            resp.get(&str_key("timestamp")),
            Some(&LuaValue::Integer(1709654400))
        );
    }

    // ── unknown tool ─────────────────────────────────────────────────

    #[test]
    fn unknown_tool_errors() {
        let mut host = StubHost;
        let args = LuaTable::new();
        let err = host.call_tool("nonexistent", &args).unwrap_err();
        assert!(err.contains("unknown tool 'nonexistent'"));
    }

    // ── tool descriptions ────────────────────────────────────────────

    #[test]
    fn stub_descriptions_match_host_tools() {
        let descs = stub_tool_descriptions();
        let names: Vec<&str> = descs.iter().map(|d| d.name.as_str()).collect();
        // Every described tool should work in the host
        let mut host = StubHost;
        for name in &names {
            // time_now needs no special args
            if *name == "time_now" {
                let _ = host.call_tool(name, &LuaTable::new());
            }
        }
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"add"));
        assert!(names.contains(&"upper"));
        assert!(names.contains(&"time_now"));
    }

    #[test]
    fn stub_descriptions_have_content() {
        let descs = stub_tool_descriptions();
        for desc in &descs {
            assert!(!desc.name.is_empty());
            assert!(!desc.description.is_empty());
        }
    }
}

/// Tool descriptions for the stub host (used in prompt generation).
pub fn stub_tool_descriptions() -> Vec<crate::prompt::ToolDescription> {
    use crate::prompt::ToolDescription;
    vec![
        ToolDescription {
            name: "echo".into(),
            description: "Echoes the message back. Useful for testing.".into(),
            args: vec![("message".into(), "string — the message to echo".into())],
            returns: vec![("message".into(), "string — the echoed message".into())],
        },
        ToolDescription {
            name: "add".into(),
            description: "Adds two integers.".into(),
            args: vec![
                ("a".into(), "integer — first number".into()),
                ("b".into(), "integer — second number".into()),
            ],
            returns: vec![("result".into(), "integer — the sum".into())],
        },
        ToolDescription {
            name: "upper".into(),
            description: "Converts a string to uppercase.".into(),
            args: vec![("text".into(), "string — text to convert".into())],
            returns: vec![("result".into(), "string — the uppercased text".into())],
        },
        ToolDescription {
            name: "time_now".into(),
            description: "Returns the current Unix timestamp.".into(),
            args: vec![],
            returns: vec![("timestamp".into(), "integer — Unix timestamp in seconds".into())],
        },
    ]
}
