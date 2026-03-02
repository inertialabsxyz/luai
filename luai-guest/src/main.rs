use luai::{OracleTape, compiler::CompiledProgram, types::value::LuaValue};
use luai::{
    bytecode::verify,
    host::tape::TapeHost,
    vm::engine::{Vm, VmConfig},
    zkvm::commitment::compute_public_inputs,
};

fn main() {
    let program = CompiledProgram {
        prototypes: vec![],
        program_hash: [0u8; 32],
    };

    let vm_config = VmConfig {
        gas_limit: 0,
        memory_limit_bytes: 0,
        max_call_depth: 0,
        max_tool_calls: 0,
        max_tool_bytes_in: 0,
        max_tool_bytes_out: 0,
        max_output_bytes: 0,
    };

    let oracle_tape = OracleTape { entries: vec![] };

    let input_value = LuaValue::Nil;
    verify(&program).expect("bytecode verification failed");

    let tape_host = TapeHost::new(oracle_tape.clone());
    let mut vm = Vm::new(vm_config.clone(), tape_host);

    let output = vm
        .execute(&program, input_value.clone())
        .expect("VM execution failed");

    let public_inputs =
        compute_public_inputs(program.program_hash, &input_value, &oracle_tape, &output);

    println!("{:?}", public_inputs);
}
