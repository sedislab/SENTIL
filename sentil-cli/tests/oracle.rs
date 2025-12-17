//! The deterministic tests

use std::fs;
use std::io::Write;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::NamedTempFile;

#[test]
fn deterministic_oracle_is_bit_exact() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../benchmarks/deterministic/oracle.json"
    );
    let oracle: Value =
        serde_json::from_str(&fs::read_to_string(path).expect("read oracle.json")).expect("parse");
    let cases = oracle["deterministic"]
        .as_array()
        .expect("the deterministic array");

    for case in cases {
        let id = case["id"].as_str().unwrap_or("?");
        let formula = case["formula"].as_str().expect("formula");
        let length = case["length"].as_u64().expect("length") as usize;
        let signals = case["signals"].as_array().expect("signals");
        let expected: Vec<f64> = case["expected"]
            .as_array()
            .expect("expected")
            .iter()
            .map(|v| parse_f64(v.as_str().unwrap()))
            .collect();

        let trace = write_trace(signals, length);
        let output = Command::cargo_bin("sentil")
            .unwrap()
            .args([
                "check",
                "-f",
                formula,
                "-t",
                trace.path().to_str().unwrap(),
                "--semantics",
                "discrete",
                "--signal",
            ])
            .output()
            .unwrap();

        let code = output.status.code();
        assert!(
            code == Some(0) || code == Some(10),
            "case {id} ({formula}) exited {code:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let got: Vec<f64> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(parse_f64)
            .collect();
        assert_eq!(got.len(), expected.len(), "case {id}: signal length");
        for (i, (g, e)) in got.iter().zip(&expected).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "case {id} sample {i}: got {g}, expected {e}"
            );
        }
    }
}

fn write_trace(signals: &[Value], length: usize) -> NamedTempFile {
    let names: Vec<&str> = signals.iter().map(|s| s["name"].as_str().unwrap()).collect();
    let columns: Vec<Vec<&str>> = signals
        .iter()
        .map(|s| {
            s["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect()
        })
        .collect();
    let mut csv = format!("time,{}\n", names.join(","));
    for row in 0..length {
        csv.push_str(&row.to_string());
        for column in &columns {
            csv.push(',');
            csv.push_str(column[row]);
        }
        csv.push('\n');
    }
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(csv.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

fn parse_f64(text: &str) -> f64 {
    match text.trim().to_ascii_lowercase().as_str() {
        "inf" | "+inf" | "infinity" => f64::INFINITY,
        "-inf" | "-infinity" => f64::NEG_INFINITY,
        "nan" => f64::NAN,
        other => other.parse().unwrap_or_else(|_| panic!("not a float: {text}")),
    }
}