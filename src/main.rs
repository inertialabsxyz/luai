use luai::{
    bytecode, compiler, parser,
    types::value::LuaValue,
    vm::{
        engine::{NoopHost, Vm, VmConfig},
        gas::VmError,
    },
};
use std::{
    env, fs,
    io::{self, Read},
};

fn source_line(source: &str, line: u32) -> &str {
    source
        .lines()
        .nth(line.saturating_sub(1) as usize)
        .unwrap_or("")
}

fn run(source: &str) -> Result<(), VmError> {
    let ast = parser::parse(source).map_err(|e| {
        VmError::RuntimeError(LuaValue::String(luai::types::value::LuaString::from_str(
            &format!("parse error: {e:?}"),
        )))
    })?;
    let program = compiler::compile(&ast).map_err(|e| {
        VmError::RuntimeError(LuaValue::String(luai::types::value::LuaString::from_str(
            &format!("compile error: {e:?}"),
        )))
    })?;
    bytecode::verify(&program).map_err(|e| {
        VmError::RuntimeError(LuaValue::String(luai::types::value::LuaString::from_str(
            &format!("verify error: {e:?}"),
        )))
    })?;

    let mut vm = Vm::new(VmConfig::default(), NoopHost);
    let output = vm.execute(&program, LuaValue::Nil)?;

    for msg in &output.logs {
        println!("{msg}");
    }
    if !matches!(output.return_value, LuaValue::Nil) {
        println!("=> {}", output.return_value);
    }
    eprintln!(
        "[gas: {}, mem: {} bytes]",
        output.gas_used, output.memory_used
    );
    Ok(())
}

fn main() {
    let source = if let Some(path) = env::args().nth(1) {
        fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("error reading {path}: {e}");
            std::process::exit(1);
        })
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap();
        buf
    };

    if let Err(e) = run(&source) {
        use luai::vm::gas::VmError;
        match e {
            VmError::WithLine(line, inner) => {
                let text = source_line(&source, line).trim();
                eprintln!("runtime error at line {line}: {inner:?}");
                eprintln!("  --> {text}");
            }
            other => eprintln!("runtime error: {other:?}"),
        }
        std::process::exit(1);
    }
}
