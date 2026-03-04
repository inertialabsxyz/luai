//! `Prover` — dry-run and proof generation for Lua agent executions.

use luai::{
    compiler::proto::CompiledProgram,
    host::tape::OracleTape,
    types::value::LuaValue,
    vm::engine::{HostInterface, Vm, VmConfig, VmOutput},
    zkvm::{
        commitment::{PublicInputs, compute_public_inputs},
        guest_input::GuestInput,
    },
};
use serde::{Deserialize, Serialize};

/// Result of a dry run: the VM output and the oracle tape built from the transcript.
#[derive(Debug, Serialize, Deserialize)]
pub struct DryRunResult {
    pub output: VmOutput,
    pub oracle_tape: OracleTape,
    pub public_inputs: PublicInputs,
}

/// Executes Lua programs and (optionally) proves executions in the zkVM.
pub struct Prover<H: HostInterface> {
    config: VmConfig,
    host: H,
    tool_names: Vec<String>,
}

impl<H: HostInterface> Prover<H> {
    /// Create a new prover with the given VM config, live host, and registered tool names.
    pub fn new(config: VmConfig, host: H, tool_names: Vec<String>) -> Self {
        Prover {
            config,
            host,
            tool_names,
        }
    }

    /// Execute the program with the live host, record a transcript, and build an oracle tape.
    ///
    /// This is "phase 1" of the two-phase execution model. The result contains
    /// the oracle tape needed for the zkVM replay.
    pub fn dry_run(
        self,
        program: &CompiledProgram,
        input: LuaValue,
    ) -> Result<DryRunResult, luai::VmError> {
        let mut vm = Vm::new(self.config.clone(), self.host);
        let output = vm.execute(program, input.clone())?;

        let oracle_tape = OracleTape::from_records(&output.transcript);
        let public_inputs =
            compute_public_inputs(program.program_hash, &input, &oracle_tape, &output);

        Ok(DryRunResult {
            output,
            oracle_tape,
            public_inputs,
        })
    }

    /// Build a `GuestInput` from a dry-run result (for passing to the zkVM).
    pub fn build_guest_input(
        &self,
        program: CompiledProgram,
        input: LuaValue,
        dry_run: &DryRunResult,
    ) -> GuestInput {
        GuestInput::new(
            program,
            input,
            dry_run.oracle_tape.clone(),
            self.config.clone(),
            self.tool_names.clone(),
        )
    }
}
