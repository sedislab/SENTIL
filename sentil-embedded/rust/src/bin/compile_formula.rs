//! Compiles a formula into the compact byte form the smallest boards load
//! without shipping the parser.
//!
//! Run it on a workstation, where any mistake in the formula is caught before
//! the board is flashed. By default it prints a ready-to-paste C array on stdout
//! and the packed variable order on stderr; with `-o <file>` it writes the raw
//! bytes instead.
//!
//! ```text
//! sentil-compile-formula "always[0, 10](speed < 5)"
//! sentil-compile-formula "x > 0" -o formula.bin
//! ```

use std::process::ExitCode;

use sentil::StreamMonitor;
use sentil_embedded::codec;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (formula, output) = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("usage: sentil-compile-formula \"<formula>\" [-o <file>]");
            return ExitCode::from(2);
        }
    };

    let parsed = match sentil::Formula::parse(&formula) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("could not parse the formula: {e}");
            return ExitCode::from(2);
        }
    };

    let bytes = codec::encode(&parsed);
    print_packed_order(&parsed);

    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, &bytes) {
                eprintln!("could not write {path}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("wrote {} bytes to {path}", bytes.len());
        }
        None => print_c_array(&bytes),
    }
    ExitCode::SUCCESS
}

fn parse_args(args: &[String]) -> Result<(String, Option<String>), String> {
    let mut formula = None;
    let mut output = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = Some(args.get(i).ok_or("missing path after -o")?.clone());
            }
            other if formula.is_none() => formula = Some(other.to_string()),
            other => return Err(format!("unexpected argument `{other}`")),
        }
        i += 1;
    }
    Ok((formula.ok_or("no formula given")?, output))
}

fn print_packed_order(formula: &sentil::Formula) {
    let Ok(monitor) = StreamMonitor::from_formula(formula) else {
        return;
    };
    eprintln!("packed order ({} variables):", monitor.variable_count());
    for name in formula.variables() {
        if let Some(index) = monitor.symbol_index(&name) {
            eprintln!("  [{index}] {name}");
        }
    }
}

fn print_c_array(bytes: &[u8]) {
    println!("static const unsigned char SENTIL_FORMULA[] = {{");
    for chunk in bytes.chunks(12) {
        let row: Vec<String> = chunk.iter().map(|b| format!("0x{b:02x}")).collect();
        println!("    {},", row.join(", "));
    }
    println!("}};");
    println!("static const unsigned int SENTIL_FORMULA_LEN = {};", bytes.len());
}