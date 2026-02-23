pub mod host;
pub mod types;
pub mod vm;
pub mod parser;
pub mod compiler;
pub mod bytecode;

pub use vm::engine::{Vm, VmConfig, VmOutput, HostInterface, NoopHost};
pub use vm::gas::{GasMeter, VmError};
pub use vm::memory::MemoryMeter;
pub use host::transcript::{ToolCallRecord, ToolCallStatus, Transcript};

pub fn execute(
    program: &compiler::proto::CompiledProgram,
    input: types::value::LuaValue,
    config: VmConfig,
) -> Result<VmOutput, VmError> {
    let mut vm = Vm::new(config, NoopHost);
    vm.execute(program, input)
}
