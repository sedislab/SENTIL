//! Compiles a formula into the compact byte form the smallest boards load
//! without shipping the parser.
//!
//! ```text
//! sentil-compile-formula "always[0, 10](speed < 5)"
//! sentil-compile-formula "x > 0" -o formula.bin
//! sentil-compile-formula --list-specs
//! sentil-compile-formula --spec controls/overshoot --param limit=1.2 -o formula.bin
//! ```

use std::process::ExitCode;

use sentil::StreamMonitor;
use sentil_embedded::codec;

#[derive(Default)]
struct Args {
    formula: Option<String>,
    output: Option<String>,
    spec: Option<String>,
    variant: Option<String>,
    params: Vec<(String, f64)>,
    list_specs: bool,
}

fn main() -> ExitCode {
    let args = match parse_args(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("usage: sentil-compile-formula \"<formula>\" [-o <file>]");
            eprintln!("       sentil-compile-formula --spec <name> [--variant <v>] [--param k=v] [-o <file>]");
            eprintln!("       sentil-compile-formula --list-specs");
            return ExitCode::from(2);
        }
    };

    if args.list_specs {
        return list_specs();
    }

    let formula = match resolve_formula(&args) {
        Ok(formula) => formula,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let bytes = codec::encode(&formula);
    print_packed_order(&formula);
    match args.output {
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

fn resolve_formula(args: &Args) -> Result<sentil::Formula, String> {
    if let Some(name) = &args.spec {
        return formula_from_spec(name, args.variant.as_deref(), &args.params);
    }
    let text = args.formula.as_ref().ok_or("no formula or spec given")?;
    sentil::Formula::parse(text).map_err(|e| format!("could not parse the formula: {e}"))
}

#[cfg(feature = "specs")]
fn formula_from_spec(name: &str, variant: Option<&str>, params: &[(String, f64)]) -> Result<sentil::Formula, String> {
    let registry = sentil::SpecRegistry::default();
    let mut builder = registry.builder(name).map_err(|e| format!("spec `{name}`: {e}"))?;
    if let Some(variant) = variant {
        builder = builder.with_variant(variant).map_err(|e| e.to_string())?;
    }
    for (key, value) in params {
        builder = builder.with_param(key, *value).map_err(|e| e.to_string())?;
    }
    let formula = builder.build_formula().map_err(|e| e.to_string())?;
    if matches!(formula, sentil::Formula::Probabilistic(..)) {
        return Err(format!(
            "spec `{name}` resolves to a probabilistic formula, which a board cannot decide; pick a deterministic variant"
        ));
    }
    Ok(formula)
}

#[cfg(not(feature = "specs"))]
fn formula_from_spec(_name: &str, _variant: Option<&str>, _params: &[(String, f64)]) -> Result<sentil::Formula, String> {
    Err("this build has no spec library; rebuild with --features specs".to_string())
}

#[cfg(feature = "specs")]
fn list_specs() -> ExitCode {
    for name in sentil::SpecRegistry::default().available() {
        println!("{name}");
    }
    ExitCode::SUCCESS
}

#[cfg(not(feature = "specs"))]
fn list_specs() -> ExitCode {
    eprintln!("this build has no spec library; rebuild with --features specs");
    ExitCode::from(2)
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                out.output = Some(args.get(i).ok_or("missing path after -o")?.clone());
            }
            "--spec" => {
                i += 1;
                out.spec = Some(args.get(i).ok_or("missing name after --spec")?.clone());
            }
            "--variant" => {
                i += 1;
                out.variant = Some(args.get(i).ok_or("missing name after --variant")?.clone());
            }
            "--param" => {
                i += 1;
                let pair = args.get(i).ok_or("missing k=v after --param")?;
                let (key, value) = pair.split_once('=').ok_or("--param needs the form key=value")?;
                let value = value.parse::<f64>().map_err(|_| format!("`{value}` is not a number"))?;
                out.params.push((key.to_string(), value));
            }
            "--list-specs" => out.list_specs = true,
            other if out.formula.is_none() && out.spec.is_none() => out.formula = Some(other.to_string()),
            other => return Err(format!("unexpected argument `{other}`")),
        }
        i += 1;
    }
    Ok(out)
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