# luai

A deterministic, sandboxed Lua virtual machine for agentic workloads.

Scripts run as single-shot programs: they receive an input object, may invoke host-provided tools, and return a structured result. Given the same bytecode, inputs, and tool responses, execution is always identical — making transcripts verifiable and zk-provable.

## Key properties

- **Deterministic** — no floats, no hash randomization, canonical table iteration order
- **Sandboxed** — no filesystem, network, or OS access; tools are the only external interface
- **Bounded** — gas, memory, depth, and output limits guarantee termination
- **Integer-only arithmetic** — signed 64-bit; use fixed-point for fractional values

## Status

All 10 implementation phases complete. 565 tests passing.

| Phase | Component | Status |
|-------|-----------|--------|
| 1 | `types/table.rs` — LuaTable with array+hash storage, deterministic iteration | Complete |
| 2 | `types/value.rs` — LuaValue, LuaString, arithmetic, comparison, coercion | Complete |
| 3 | `parser/` — Lexer + recursive-descent parser for the Lua subset | Complete |
| 4 | `bytecode/` — Instruction set, Program, constants | Complete |
| 5 | `bytecode/verifier.rs` — Stack-depth and operand verification | Complete |
| 6 | `compiler/` — AST → bytecode code generation | Complete |
| 7 | `vm/` — Register-based engine, gas/memory meters, builtins | Complete |
| 8 | `host/` — Tool registry, transcript, canonical JSON serializer | Complete |
| 9 | `host/tape.rs` — Oracle tape for deterministic replay / zk-proving | Complete |
| 10 | `json` module — `json.encode` / `json.decode`, full correctness + tests | Complete |

## Architecture

```
Lua source
    │
    ▼
parser/          — lexer + recursive-descent parser → AST
    │
    ▼
compiler/        — AST → bytecode (Program + constants + protos)
    │
    ▼
bytecode/        — instruction set; verifier checks stack depth
    │
    ▼
vm/engine.rs     — executes bytecode; gas + memory metering
    │             builtins: string, math, table, json, type, pcall, …
    │
    ▼
host/            — HostInterface for tool calls; ToolRegistry enforces
                   quotas and records a Transcript; canonical JSON for
                   deterministic serialization; OracleTape for replay
```

## Standard library

| Module | Functions |
|--------|-----------|
| `string` | `len`, `sub`, `find`, `upper`, `lower`, `rep`, `byte`, `char`, `format` |
| `math` | `abs`, `min`, `max`, `scale_div` |
| `table` | `insert`, `remove`, `concat`, `move`, `sort` |
| `json` | `encode`, `decode` |
| top-level | `type`, `tostring`, `tonumber`, `select`, `unpack`, `pcall`, `error`, `log`, `print`, `pairs_sorted`, `ipairs` |

## Oracle tape / replay

Every tool call is recorded in a `Transcript`. The transcript can be converted to an `OracleTape` whose `commitment_hash()` (SHA-256 over all responses) identifies the exact execution. Replaying against the tape via `TapeHost` reproduces the same return value, gas, and memory usage deterministically — suitable for zk-proof generation.

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

See `Agentic Lua VM Specification v1.md` for the full design and `impl/` for per-phase implementation notes.
