# luai

A deterministic, sandboxed Lua virtual machine for agentic workloads.

Scripts run as single-shot programs: they receive an input object, may invoke host-provided tools, and return a structured result. Given the same bytecode, inputs, and tool responses, execution is always identical — making transcripts verifiable and zk-provable.

## Key properties

- **Deterministic** — no floats, no hash randomization, canonical table iteration order
- **Sandboxed** — no filesystem, network, or OS access; tools are the only external interface
- **Bounded** — gas, memory, depth, and output limits guarantee termination
- **Integer-only arithmetic** — signed 64-bit; use fixed-point for fractional values

## Status

Early implementation. Currently working through the implementation phases defined in the spec.

| Phase | Component | Status |
|-------|-----------|--------|
| 1 | `types/table.rs` | In progress |
| 2 | `types/value.rs` | Stub |
| 3–10 | Parser, compiler, VM, host | Not started |

See `Agentic Lua VM Specification v1.md` for the full design and `impl/` for per-phase implementation notes.
