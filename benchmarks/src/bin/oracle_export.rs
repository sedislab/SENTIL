//! Same baseline so that all the tools can check against the same formulas.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use sentil_benchmarks::oracle::{CANONICAL, DETERMINISTIC};

fn token(v: f64) -> String {
    if v.is_nan() {
        "nan".to_string()
    } else if v.is_infinite() {
        if v > 0.0 { "inf".to_string() } else { "-inf".to_string() }
    } else {
        format!("{v}")
    }
}

fn array(values: &[f64]) -> String {
    let mut s = String::from("[");
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        let _ = write!(s, "\"{}\"", token(*v));
    }
    s.push(']');
    s
}

fn main() {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(
        "  \"note\": \"Generated from benchmarks/src/oracle.rs.\",\n",
    );

    out.push_str("  \"deterministic\": [\n");
    for (ci, case) in DETERMINISTIC.iter().enumerate() {
        out.push_str("    {");
        let _ = write!(out, "\"id\": \"{}\", ", case.id);
        let _ = write!(out, "\"formula\": {:?}, ", case.formula);
        let _ = write!(out, "\"length\": {}, ", case.expected.len());
        out.push_str("\"signals\": [");
        for (si, (name, vals)) in case.signals.iter().enumerate() {
            if si > 0 {
                out.push_str(", ");
            }
            let _ = write!(out, "{{\"name\": \"{}\", \"values\": {}}}", name, array(vals));
        }
        let _ = write!(out, "], \"expected\": {}}}", array(case.expected));
        out.push_str(if ci + 1 < DETERMINISTIC.len() { ",\n" } else { "\n" });
    }
    out.push_str("  ],\n");

    out.push_str("  \"canonical\": [\n");
    for (ci, (formula, expected)) in CANONICAL.iter().enumerate() {
        let _ = write!(
            out,
            "    {{\"formula\": {formula:?}, \"size\": 2001, \"expected\": \"{}\"}}",
            token(*expected)
        );
        out.push_str(if ci + 1 < CANONICAL.len() { ",\n" } else { "\n" });
    }
    out.push_str("  ]\n}");

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("deterministic");
    fs::create_dir_all(&dir).expect("create benchmarks/deterministic");
    let path = dir.join("oracle.json");
    fs::write(&path, out).expect("write oracle.json");
    println!("wrote {} cases to {}", DETERMINISTIC.len(), path.display());
}