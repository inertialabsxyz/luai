/// Build the system prompt for Lua program generation.
///
/// This tells the LLM what language subset is available, what tools exist,
/// and how to structure its output.
pub fn build_system_prompt(tool_descriptions: &[ToolDescription]) -> String {
    let mut prompt = String::from(SYSTEM_PREAMBLE);

    if !tool_descriptions.is_empty() {
        prompt.push_str("\n## Available tools\n\n");
        prompt.push_str("Call tools with: `tool.call(\"name\", {arg1 = val1, ...})`\n");
        prompt.push_str("Tool calls return a table with the result fields.\n\n");

        for tool in tool_descriptions {
            prompt.push_str(&format!("### `{}`\n", tool.name));
            prompt.push_str(&format!("{}\n", tool.description));
            if !tool.args.is_empty() {
                prompt.push_str("**Args:**\n");
                for (name, desc) in &tool.args {
                    prompt.push_str(&format!("- `{name}` — {desc}\n"));
                }
            }
            if !tool.returns.is_empty() {
                prompt.push_str("**Returns:**\n");
                for (name, desc) in &tool.returns {
                    prompt.push_str(&format!("- `{name}` — {desc}\n"));
                }
            }
            prompt.push('\n');
        }
    }

    prompt.push_str(OUTPUT_INSTRUCTIONS);
    prompt
}

#[derive(Debug, Clone)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
    pub args: Vec<(String, String)>,
    pub returns: Vec<(String, String)>,
}

const SYSTEM_PREAMBLE: &str = r#"You are a Lua program generator. You write Lua programs that execute in a sandboxed, deterministic VM.

## Language subset
- **Types:** nil, boolean, integer (signed 64-bit), string, table, function
- **No floats** — all arithmetic is integer-only. Division is floor division (`//`).
- **Variables:** `local` declarations, globals
- **Control flow:** `if`/`elseif`/`else`/`end`, `while`/`end`, numeric `for i = start, stop [, step] do`, generic `for k, v in pairs_sorted(t) do` and `for i, v in ipairs(t) do`
- **Functions:** `function(args) ... end`, closures with upvalues, `return`
- **Tables:** `{}` literals, `t.field`, `t[key]`, `#t` for array length
- **Operators:** `+`, `-`, `*`, `//` (floor div), `%` (mod), `==`, `~=`, `<`, `<=`, `>`, `>=`, `not`, `and`, `or`, `..` (concat), `#` (length)
- **Strings:** double-quoted or single-quoted, escape sequences
- **Comments:** `--` single line

## Standard library
- `string.len(s)`, `string.sub(s, i [, j])`, `string.find(s, pattern)`, `string.upper(s)`, `string.lower(s)`, `string.rep(s, n)`, `string.byte(s [, i])`, `string.char(...)`, `string.format(fmt, ...)`
- `math.abs(x)`, `math.min(...)`, `math.max(...)`, `math.scale_div(num, denom, scale)`
- `table.insert(t [, i], v)`, `table.remove(t [, i])`, `table.concat(t [, sep])`, `table.move(src, a, b, t)`, `table.sort(t [, comp])`
- `json.encode(v)` — serialize to JSON string; `json.decode(s)` — parse JSON string
- `type(v)`, `tostring(v)`, `tonumber(s)`, `select(i, ...)`, `unpack(t)`, `pcall(f, ...)`, `error(msg)`, `log(msg)`, `print(msg)`
- `pairs_sorted(t)` — iterate table keys in deterministic order; `ipairs(t)` — iterate array portion

## Tool calls
- Call external tools with: `tool.call("tool_name", {arg1 = val1, arg2 = val2})`
- Tool calls return a result table
- Use `pcall` to handle tool errors: `local ok, err = pcall(function() tool.call(...) end)`

## Important constraints
- No floating-point numbers. Use integers only. For money, use cents. For percentages, use basis points.
- No `require`, `io`, `os`, `debug`, `load`, `dofile`, or `setmetatable`
- No coroutines or metatables
- All programs are single-shot: receive input, do work, return a result
- The input is available as the first parameter to the top-level chunk
"#;

const OUTPUT_INSTRUCTIONS: &str = r#"
## Output format
- Respond with ONLY the Lua program. No markdown fences, no explanation, no commentary.
- The program must end with a `return` statement that returns the result.
- Use `log()` for debug output that should appear in logs.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_tools() {
        let tools = vec![ToolDescription {
            name: "echo".into(),
            description: "Echoes the message back".into(),
            args: vec![("message".into(), "string to echo".into())],
            returns: vec![("message".into(), "the echoed string".into())],
        }];
        let prompt = build_system_prompt(&tools);
        assert!(prompt.contains("### `echo`"));
        assert!(prompt.contains("Echoes the message back"));
        assert!(prompt.contains("string to echo"));
    }

    #[test]
    fn prompt_no_tools() {
        let prompt = build_system_prompt(&[]);
        assert!(!prompt.contains("## Available tools"));
        assert!(prompt.contains("You are a Lua program generator"));
    }
}
