//! luai-prover — host-side prover and verifier for zkVM proofs.
//!
//! This crate provides:
//! - `Prover`: executes the Lua program with a live host (dry run), then replays
//!   it inside the zkVM to produce a proof.
//! - `Verifier`: verifies a proof and extracts the `PublicInputs`.
//!
//! NOTE: The actual zkVM proof generation requires the RISC Zero toolchain and
//! is stubbed out here. The `dry_run` functionality works without RISC Zero.

pub mod prover;
pub mod verifier;

pub use prover::Prover;
pub use verifier::Verifier;
