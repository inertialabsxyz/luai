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
