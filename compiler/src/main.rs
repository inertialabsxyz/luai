use std::{
    env,
    fs::{self, File},
    io::{self, Read},
};

use luai::{bytecode, compiler, parser};

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

    let out_path = match env::args().nth(2) {
        Some(p) => p,
        None => String::from("compiled.json"),
    };

    let ast = match parser::parse(&source) {
        Ok(v) => v,
        Err(e) => {
            eprint!("parse error: {e:?}");
            return;
        }
    };

    let program = match compiler::compile(&ast) {
        Ok(v) => v,
        Err(e) => {
            eprint!("compile error: {e:?}");
            return;
        }
    };

    if let Err(e) = bytecode::verify(&program) {
        eprintln!("verification error: {e:?}");
        return;
    }

    let out_file = File::create(&out_path).unwrap();
    serde_json::to_writer(out_file, &program).unwrap();
    println!("File written - {}", out_path);
}
