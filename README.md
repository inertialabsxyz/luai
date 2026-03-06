# luai

A deterministic, sandboxed Lua virtual machine for agentic workloads.

Scripts run as single-shot programs: they receive an input object, may invoke host-provided tools, and return a structured result. Given the same bytecode, inputs, and tool responses, execution is always identical — making transcripts verifiable and zk-provable.

## Key properties

- **Deterministic** — no floats, no hash randomization, canonical table iteration order
- **Sandboxed** — no filesystem, network, or OS access; tools are the only external interface
- **Bounded** — gas, memory, depth, and output limits guarantee termination
- **Integer-only arithmetic** — signed 64-bit; use fixed-point for fractional values
- **ZK-provable** — two-phase execution model with OpenVM integration

## Workspace

| Crate | Purpose |
|-------|---------|
| `luai` | Core library — parser, compiler, bytecode verifier, VM, host, oracle tape |
| `compiler` | CLI: compile Lua source → verified bytecode JSON |
| `prover` | CLI: dry-run compiled programs, produce oracle tapes and public inputs |
| `openvm` | OpenVM guest + encoder for zk proof generation and verification |
| `orchestrator` | LLM-driven agentic pipeline — accepts a task, generates and executes Lua |

## Architecture

```
Lua source
    │
    ▼
parser/          — lexer + recursive-descent parser → AST
    │
    ▼
compiler/        — AST → bytecode (prototypes + constants + program hash)
    │
    ▼
bytecode/        — instruction set; verifier checks stack depth & operands
    │
    ▼
vm/engine.rs     — register-based execution; gas + memory metering
    │             builtins: string, math, table, json, type, pcall, …
    │
    ▼
host/            — HostInterface for tool calls; transcript recording;
                   canonical JSON; OracleTape for deterministic replay
```

### Proving pipeline

```
1. Compile       luai-compiler source.lua compiled.json
2. Dry run       luai-prover compiled.json dry_result.json
                 → executes with live host, records oracle tape
                 → computes public inputs (SHA-256 commitments)
3. Encode        luai-openvm-encoder compiled.json dry_result.json
                 → serializes guest input for OpenVM
4. Prove         openvm guest replays execution against oracle tape
                 → verifies public inputs match
```

Public inputs commit to: program hash, input hash, tool responses hash, output hash.

## Orchestrator

The orchestrator connects an LLM (Claude) to the luai VM, forming an agentic pipeline:

1. User provides a natural-language task
2. The LLM generates a Lua program to accomplish it
3. The program is compiled, verified, and executed in the sandboxed VM
4. Tool calls reach real external services (HTTP, KV store, LLM sub-queries)
5. On error, the LLM retries with error context (up to N attempts)
6. A full execution report is produced with verification hashes

### Usage

```
export ANTHROPIC_API_KEY=sk-...
cargo run -p luai-orchestrator -- "fetch the top hacker news story title"
```

### Options

| Flag | Description |
|------|-------------|
| `--json` | Machine-readable JSON output with full transcript and verification hashes |
| `--verbose` / `-v` | Show system prompt and raw LLM responses |
| `--model <model>` | Claude model to use (default: `claude-sonnet-4-20250514`) |
| `--max-retries <n>` | Max retry attempts on errors (default: 3) |

### Available tools

Programs running in the VM can call these tools via `tool.call(name, args)`:

| Tool | Description |
|------|-------------|
| `http_get` | GET a URL, returns `{status, body}` |
| `http_post` | POST JSON to a URL, returns `{status, body}` |
| `kv_get` | Read from an in-memory key-value store |
| `kv_set` | Write to an in-memory key-value store |
| `llm_query` | Call the LLM for fuzzy reasoning sub-tasks |
| `time_now` | Current Unix timestamp |

### Execution report

Every run produces a report with:
- The generated Lua program
- Return value and logs
- Full tool call transcript (name, args, response, bytes)
- Resource usage (gas, memory, tool calls) with limits
- SHA-256 verification hashes (program, oracle tape, output)

The `--json` flag outputs all of this as structured JSON for machine consumption.

## Standard library

| Module | Functions |
|--------|-----------|
| `string` | `len`, `sub`, `find`, `upper`, `lower`, `rep`, `byte`, `char`, `format` |
| `math` | `abs`, `min`, `max`, `scale_div` |
| `table` | `insert`, `remove`, `concat`, `move`, `sort` |
| `json` | `encode`, `decode` |
| top-level | `type`, `tostring`, `tonumber`, `select`, `unpack`, `pcall`, `error`, `log`, `print`, `pairs_sorted`, `ipairs` |

## Resource limits (defaults)

| Limit | Default |
|-------|---------|
| Gas | 10,000,000 |
| Memory | 64 MB |
| Call depth | 200 |
| Tool calls | 64 |
| Bytes in per call | 1 MB |
| Bytes out per call | 64 KB |
| JSON / string length | 64 KB |
| Table / call nesting depth | 32 |
