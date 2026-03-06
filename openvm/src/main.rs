mod encoder;

use encoder::OpenVMInput;
use luai::{
    bytecode::verify,
    host::tape::TapeHost,
    types::value::LuaValue,
    vm::engine::{Vm, VmConfig},
    zkvm::commitment::compute_public_inputs,
};

fn main() {
    let input = openvm::io::read::<OpenVMInput>();
    let program = input.compiled_program;
    let dry_run_result = input.dry_run_result;

    let vm_config = VmConfig::default();

    let input_value = LuaValue::Nil;
    verify(&program).expect("bytecode verification failed");

    let tape_host = TapeHost::new(dry_run_result.oracle_tape.clone());
    let mut vm = Vm::new(vm_config.clone(), tape_host);

    let output = vm
        .execute(&program, input_value.clone())
        .expect("VM execution failed");

    let public_inputs = compute_public_inputs(
        program.program_hash,
        &input_value,
        &dry_run_result.oracle_tape,
        &output,
    );

    assert!(public_inputs == dry_run_result.public_inputs);
    openvm::io::reveal_bytes32(public_inputs.program_hash);
    openvm::io::reveal_bytes32(public_inputs.input_hash);
    openvm::io::reveal_bytes32(public_inputs.tool_responses_hash);
    openvm::io::reveal_bytes32(public_inputs.output_hash);
}
