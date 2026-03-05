use luai::{
    bytecode,
    compiler::{self, proto::CompiledProgram},
    host::transcript::ToolCallStatus,
    parser,
    types::value::LuaValue,
    vm::{
        engine::{HostInterface, Vm, VmConfig, VmOutput},
        gas::VmError,
    },
};

/// Result of a successful pipeline execution.
#[derive(Debug)]
pub struct PipelineResult {
    pub source: String,
    pub output: VmOutput,
    pub attempts: usize,
}

/// Errors from the compile → verify → execute pipeline.
#[derive(Debug)]
#[allow(dead_code)]
pub enum PipelineError {
    Parse(String),
    Compile(String),
    Verify(String),
    Runtime(String, VmError),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::Parse(msg) => write!(f, "Parse error: {msg}"),
            PipelineError::Compile(msg) => write!(f, "Compile error: {msg}"),
            PipelineError::Verify(msg) => write!(f, "Verify error: {msg}"),
            PipelineError::Runtime(msg, _) => write!(f, "Runtime error: {msg}"),
        }
    }
}

/// Compile Lua source to a verified program.
pub fn compile_and_verify(source: &str) -> Result<CompiledProgram, PipelineError> {
    let ast = parser::parse(source).map_err(|e| PipelineError::Parse(format!("{e:?}")))?;
    let program =
        compiler::compile(&ast).map_err(|e| PipelineError::Compile(format!("{e:?}")))?;
    bytecode::verify(&program).map_err(|e| PipelineError::Verify(format!("{e:?}")))?;
    Ok(program)
}

/// Execute a compiled program with the given host and config.
pub fn execute<H: HostInterface>(
    program: &CompiledProgram,
    input: LuaValue,
    config: VmConfig,
    host: H,
) -> Result<VmOutput, PipelineError> {
    let mut vm = Vm::new(config, host);
    vm.execute(program, input).map_err(|e| {
        let msg = format_vm_error(&e);
        PipelineError::Runtime(msg, e)
    })
}

/// Format a VmError into a human-readable string for LLM feedback.
pub fn format_vm_error(err: &VmError) -> String {
    match err {
        VmError::GasExhausted => "Gas limit exceeded — program too expensive".into(),
        VmError::MemoryExhausted => "Memory limit exceeded".into(),
        VmError::CallDepthExceeded => "Call depth exceeded — too much recursion".into(),
        VmError::TypeError(msg) => format!("Type error: {msg}"),
        VmError::RuntimeError(val) => format!("Runtime error: {val}"),
        VmError::ToolError(msg) => format!("Tool error: {msg}"),
        VmError::OutputExceeded => "Output size exceeded".into(),
        VmError::WithLine(line, inner) => {
            format!("Error at line {line}: {}", format_vm_error(inner))
        }
    }
}

/// Format a PipelineError into a context string for LLM retry.
pub fn format_error_for_retry(source: &str, err: &PipelineError) -> String {
    let mut ctx = String::new();
    ctx.push_str("Your previous Lua program failed.\n\n");
    ctx.push_str("## Previous program\n```lua\n");
    ctx.push_str(source);
    ctx.push_str("\n```\n\n");
    ctx.push_str("## Error\n");
    ctx.push_str(&err.to_string());
    ctx.push_str("\n\nPlease fix the program. Respond with ONLY the corrected Lua program.");
    ctx
}

/// Format the execution output for display.
pub fn format_output(result: &PipelineResult) -> String {
    let mut out = String::new();

    out.push_str("═══ luai execution report ═══\n\n");

    out.push_str(&format!("Attempts: {}\n\n", result.attempts));

    out.push_str("── Generated program ──────────────────────────\n");
    out.push_str(&result.source);
    out.push_str("\n\n");

    out.push_str("── Result ─────────────────────────────────────\n");
    out.push_str(&format!("{}\n\n", result.output.return_value));

    if !result.output.logs.is_empty() {
        out.push_str("── Logs ───────────────────────────────────────\n");
        for msg in &result.output.logs {
            out.push_str(&format!("  {msg}\n"));
        }
        out.push('\n');
    }

    if !result.output.transcript.is_empty() {
        out.push_str("── Transcript ─────────────────────────────────\n");
        for r in &result.output.transcript {
            let args = String::from_utf8_lossy(&r.args_canonical);
            let status = match r.status {
                ToolCallStatus::Ok => format!("ok ({} bytes)", r.response_bytes),
                ToolCallStatus::Error => "error".to_string(),
            };
            out.push_str(&format!(
                "  [{}] {} args={} → {}\n",
                r.seq, r.tool_name, args, status
            ));
        }
        out.push('\n');
    }

    out.push_str("── Resource usage ─────────────────────────────\n");
    out.push_str(&format!(
        "  Gas:    {} / {}\n",
        result.output.gas_used, "10,000,000"
    ));
    out.push_str(&format!(
        "  Memory: {} bytes\n",
        result.output.memory_used
    ));
    out.push_str(&format!(
        "  Tools:  {} call(s)\n",
        result.output.transcript.len()
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::StubHost;

    // ── compile_and_verify ───────────────────────────────────────────

    #[test]
    fn compile_valid_program() {
        let program = compile_and_verify("return 42").unwrap();
        assert!(!program.prototypes.is_empty());
    }

    #[test]
    fn compile_with_tool_call() {
        let source = r#"local r = tool.call("echo", {message = "hi"})
return r.message"#;
        let program = compile_and_verify(source).unwrap();
        assert!(!program.prototypes.is_empty());
    }

    #[test]
    fn compile_parse_error() {
        let err = compile_and_verify("if then end end").unwrap_err();
        assert!(matches!(err, PipelineError::Parse(_)));
        assert!(err.to_string().contains("Parse error"));
    }

    #[test]
    fn compile_disallowed_identifier() {
        let err = compile_and_verify("local x = require('foo')").unwrap_err();
        assert!(matches!(err, PipelineError::Parse(_)));
    }

    #[test]
    fn compile_empty_source() {
        // Empty source is valid Lua — returns nil
        let program = compile_and_verify("").unwrap();
        assert!(!program.prototypes.is_empty());
    }

    // ── execute ──────────────────────────────────────────────────────

    #[test]
    fn execute_simple_return() {
        let program = compile_and_verify("return 42").unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        assert_eq!(output.return_value, LuaValue::Integer(42));
    }

    #[test]
    fn execute_with_logs() {
        let program = compile_and_verify(r#"log("hello")
log("world")
return 0"#)
            .unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        assert_eq!(output.logs, vec!["hello", "world"]);
    }

    #[test]
    fn execute_tool_call_echo() {
        let source = r#"local r = tool.call("echo", {message = "test"})
return r.message"#;
        let program = compile_and_verify(source).unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        assert_eq!(
            output.return_value,
            LuaValue::String(luai::types::value::LuaString::from_str("test"))
        );
        assert_eq!(output.transcript.len(), 1);
        assert_eq!(output.transcript[0].tool_name, "echo");
    }

    #[test]
    fn execute_tool_call_add() {
        let source = r#"local r = tool.call("add", {a = 10, b = 32})
return r.result"#;
        let program = compile_and_verify(source).unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        assert_eq!(output.return_value, LuaValue::Integer(42));
    }

    #[test]
    fn execute_tool_call_upper() {
        let source = r#"local r = tool.call("upper", {text = "hello"})
return r.result"#;
        let program = compile_and_verify(source).unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        assert_eq!(
            output.return_value,
            LuaValue::String(luai::types::value::LuaString::from_str("HELLO"))
        );
    }

    #[test]
    fn execute_tool_call_time_now() {
        let source = r#"local r = tool.call("time_now", {})
return r.timestamp"#;
        let program = compile_and_verify(source).unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        assert_eq!(output.return_value, LuaValue::Integer(1709654400));
    }

    #[test]
    fn execute_unknown_tool_error() {
        let source = r#"tool.call("nonexistent", {})"#;
        let program = compile_and_verify(source).unwrap();
        let err = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap_err();
        assert!(matches!(err, PipelineError::Runtime(_, _)));
        assert!(err.to_string().contains("Tool error"));
    }

    #[test]
    fn execute_gas_exhaustion() {
        let source = "while true do end";
        let program = compile_and_verify(source).unwrap();
        let mut config = VmConfig::default();
        config.gas_limit = 100;
        let err = execute(&program, LuaValue::Nil, config, StubHost).unwrap_err();
        assert!(err.to_string().contains("Gas limit exceeded"));
    }

    #[test]
    fn execute_multiple_tool_calls() {
        let source = r#"
local r1 = tool.call("add", {a = 1, b = 2})
local r2 = tool.call("add", {a = 3, b = 4})
return r1.result + r2.result
"#;
        let program = compile_and_verify(source).unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        assert_eq!(output.return_value, LuaValue::Integer(10));
        assert_eq!(output.transcript.len(), 2);
    }

    // ── format_vm_error ──────────────────────────────────────────────

    #[test]
    fn format_error_gas() {
        let msg = format_vm_error(&VmError::GasExhausted);
        assert!(msg.contains("Gas limit exceeded"));
    }

    #[test]
    fn format_error_memory() {
        let msg = format_vm_error(&VmError::MemoryExhausted);
        assert!(msg.contains("Memory limit exceeded"));
    }

    #[test]
    fn format_error_depth() {
        let msg = format_vm_error(&VmError::CallDepthExceeded);
        assert!(msg.contains("Call depth exceeded"));
    }

    #[test]
    fn format_error_type() {
        let msg = format_vm_error(&VmError::TypeError("bad type".into()));
        assert!(msg.contains("Type error: bad type"));
    }

    #[test]
    fn format_error_tool() {
        let msg = format_vm_error(&VmError::ToolError("tool broke".into()));
        assert!(msg.contains("Tool error: tool broke"));
    }

    #[test]
    fn format_error_output() {
        let msg = format_vm_error(&VmError::OutputExceeded);
        assert!(msg.contains("Output size exceeded"));
    }

    #[test]
    fn format_error_with_line() {
        let inner = VmError::TypeError("oops".into());
        let msg = format_vm_error(&VmError::WithLine(42, Box::new(inner)));
        assert!(msg.contains("line 42"));
        assert!(msg.contains("Type error: oops"));
    }

    // ── format_error_for_retry ───────────────────────────────────────

    #[test]
    fn retry_context_includes_source_and_error() {
        let err = PipelineError::Parse("unexpected token".into());
        let ctx = format_error_for_retry("return ???", &err);
        assert!(ctx.contains("return ???"));
        assert!(ctx.contains("Parse error"));
        assert!(ctx.contains("unexpected token"));
        assert!(ctx.contains("Please fix the program"));
    }

    // ── format_output ────────────────────────────────────────────────

    #[test]
    fn format_output_simple() {
        let program = compile_and_verify("return 42").unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        let result = PipelineResult {
            source: "return 42".into(),
            output,
            attempts: 1,
        };
        let formatted = format_output(&result);
        assert!(formatted.contains("luai execution report"));
        assert!(formatted.contains("Attempts: 1"));
        assert!(formatted.contains("return 42"));
        assert!(formatted.contains("42")); // return value
        assert!(formatted.contains("Gas:"));
        assert!(formatted.contains("Memory:"));
        assert!(formatted.contains("Tools:  0 call(s)"));
        // No logs or transcript sections for this simple program
        assert!(!formatted.contains("── Logs"));
        assert!(!formatted.contains("── Transcript"));
    }

    #[test]
    fn format_output_with_logs_and_transcript() {
        let source = r#"log("debug info")
local r = tool.call("echo", {message = "hi"})
return 0"#;
        let program = compile_and_verify(source).unwrap();
        let output = execute(&program, LuaValue::Nil, VmConfig::default(), StubHost).unwrap();
        let result = PipelineResult {
            source: source.into(),
            output,
            attempts: 2,
        };
        let formatted = format_output(&result);
        assert!(formatted.contains("Attempts: 2"));
        assert!(formatted.contains("── Logs"));
        assert!(formatted.contains("debug info"));
        assert!(formatted.contains("── Transcript"));
        assert!(formatted.contains("echo"));
        assert!(formatted.contains("Tools:  1 call(s)"));
    }
}
