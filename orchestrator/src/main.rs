mod llm;
mod pipeline;
mod prompt;
mod tools;

use clap::Parser;
use luai::{
    types::value::LuaValue,
    vm::engine::VmConfig,
};

#[derive(Parser)]
#[command(name = "luai-orchestrator")]
#[command(about = "LLM-driven agentic pipeline for the luai VM")]
struct Cli {
    /// The task to accomplish (natural language)
    task: String,

    /// Claude model to use
    #[arg(long, default_value = "claude-sonnet-4-20250514")]
    model: String,

    /// Maximum retry attempts on compile/runtime errors
    #[arg(long, default_value_t = 3)]
    max_retries: usize,

    /// Print output as JSON
    #[arg(long)]
    json: bool,

    /// Show verbose output (generated prompts, raw LLM responses)
    #[arg(long, short)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("error: ANTHROPIC_API_KEY environment variable not set");
            std::process::exit(1);
        }
    };

    let client = llm::LlmClient::new(api_key, cli.model.clone());

    // Build tool catalogue and system prompt
    let tool_descs = tools::live_tool_descriptions();
    let system_prompt = prompt::build_system_prompt(&tool_descs);

    if cli.verbose {
        eprintln!("── System prompt ──────────────────────────────");
        eprintln!("{system_prompt}");
        eprintln!("───────────────────────────────────────────────\n");
    }

    // Conversation history for multi-turn retry
    let mut messages: Vec<llm::Message> = vec![llm::Message {
        role: "user".into(),
        content: cli.task.clone(),
    }];

    let config = VmConfig::default();

    for attempt in 1..=cli.max_retries + 1 {
        // Call LLM
        eprintln!("[attempt {attempt}] generating Lua program...");

        let raw_response = match client.generate(&system_prompt, &messages) {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("error: LLM generation failed: {e}");
                std::process::exit(1);
            }
        };

        let source = llm::strip_code_fences(&raw_response);

        if cli.verbose {
            eprintln!("── LLM response (raw) ─────────────────────────");
            eprintln!("{raw_response}");
            eprintln!("── Source (cleaned) ───────────────────────────");
            eprintln!("{source}");
            eprintln!("───────────────────────────────────────────────\n");
        }

        // Compile and verify
        let program = match pipeline::compile_and_verify(&source) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[attempt {attempt}] {e}");
                if attempt <= cli.max_retries {
                    let retry_msg = pipeline::format_error_for_retry(&source, &e);
                    // Add assistant response and error feedback to conversation
                    messages.push(llm::Message {
                        role: "assistant".into(),
                        content: raw_response,
                    });
                    messages.push(llm::Message {
                        role: "user".into(),
                        content: retry_msg,
                    });
                    continue;
                }
                eprintln!("error: all attempts exhausted");
                std::process::exit(1);
            }
        };

        // Execute
        let host = tools::LiveHost::new(client.clone());
        let output = match pipeline::execute(&program, LuaValue::Nil, config.clone(), host) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[attempt {attempt}] {e}");
                if attempt <= cli.max_retries {
                    let retry_msg = pipeline::format_error_for_retry(&source, &e);
                    messages.push(llm::Message {
                        role: "assistant".into(),
                        content: raw_response,
                    });
                    messages.push(llm::Message {
                        role: "user".into(),
                        content: retry_msg,
                    });
                    continue;
                }
                eprintln!("error: all attempts exhausted");
                std::process::exit(1);
            }
        };

        // Success
        let result = pipeline::PipelineResult {
            task: cli.task.clone(),
            model: cli.model.clone(),
            source,
            output,
            config: config.clone(),
            attempts: attempt,
        };

        if cli.json {
            print_json(&result);
        } else {
            print!("{}", pipeline::format_output(&result));
        }
        return;
    }

    eprintln!("error: all attempts exhausted");
    std::process::exit(1);
}

fn print_json(result: &pipeline::PipelineResult) {
    let hashes = pipeline::compute_hashes(result);

    let transcript: Vec<serde_json::Value> = result
        .output
        .transcript
        .iter()
        .map(|r| {
            let args_str = String::from_utf8_lossy(&r.args_canonical).to_string();
            let response_str = String::from_utf8_lossy(&r.response_canonical).to_string();
            serde_json::json!({
                "seq": r.seq,
                "tool": r.tool_name,
                "args": serde_json::from_str::<serde_json::Value>(&args_str).unwrap_or(serde_json::Value::String(args_str)),
                "response": serde_json::from_str::<serde_json::Value>(&response_str).unwrap_or(serde_json::Value::String(response_str)),
                "response_hash": r.response_hash,
                "response_bytes": r.response_bytes,
                "status": format!("{:?}", r.status),
                "gas_charged": r.gas_charged,
            })
        })
        .collect();

    let json = serde_json::json!({
        "task": result.task,
        "model": result.model,
        "source": result.source,
        "attempts": result.attempts,
        "return_value": format!("{}", result.output.return_value),
        "logs": result.output.logs,
        "resource_usage": {
            "gas_used": result.output.gas_used,
            "gas_limit": result.config.gas_limit,
            "memory_used": result.output.memory_used,
            "memory_limit": result.config.memory_limit_bytes,
            "tool_calls": result.output.transcript.len(),
            "tool_call_limit": result.config.max_tool_calls,
        },
        "transcript": transcript,
        "verification": {
            "program_hash": hashes.program_hash,
            "tape_hash": hashes.tape_hash,
            "output_hash": hashes.output_hash,
        },
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
