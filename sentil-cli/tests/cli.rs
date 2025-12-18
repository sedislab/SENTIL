//! End-to-end checks of the command surface

use std::io::Write;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn sentil() -> Command {
    Command::cargo_bin("sentil").unwrap()
}

fn file(content: &str) -> NamedTempFile {
    let mut handle = NamedTempFile::new().unwrap();
    handle.write_all(content.as_bytes()).unwrap();
    handle.flush().unwrap();
    handle
}

fn path(handle: &NamedTempFile) -> String {
    handle.path().to_str().unwrap().to_string()
}

#[test]
fn check_satisfied_exits_zero() {
    let trace = file("time,x\n0,1\n1,2\n");
    sentil()
        .args(["check", "-f", "x > 0", "-t", &path(&trace)])
        .assert()
        .success()
        .stdout(predicate::str::contains("satisfied"));
}

#[test]
fn check_violated_exits_ten() {
    let trace = file("time,x\n0,-1\n");
    sentil()
        .args(["check", "-f", "x > 0", "-t", &path(&trace)])
        .assert()
        .code(10)
        .stdout(predicate::str::contains("violated"));
}

#[test]
fn check_json_carries_the_schema() {
    let trace = file("time,x\n0,1\n");
    sentil()
        .args(["check", "-f", "x > 0", "-t", &path(&trace), "-o", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\":\"1.0\""))
        .stdout(predicate::str::contains("\"verb\":\"check\""));
}

#[test]
fn json_alias_equals_output_json() {
    let trace = file("time,x\n0,1\n");
    sentil()
        .args(["check", "-f", "x > 0", "-t", &path(&trace), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\""));
}

#[test]
fn parse_error_exits_data_err() {
    let trace = file("time,x\n0,1\n");
    sentil()
        .args(["check", "-f", "always[0,5] (x > 3 &)", "-t", &path(&trace)])
        .assert()
        .code(65)
        .stderr(predicate::str::contains("sentil::parse"));
}

#[test]
fn missing_file_exits_no_input() {
    sentil()
        .args(["check", "-f", "x > 0", "-t", "/no/such/trace.csv"])
        .assert()
        .code(66);
}

#[test]
fn gpu_backend_exits_unavailable() {
    let trace = file("time,x\n0,1\n");
    sentil()
        .args(["check", "-f", "x > 0", "-t", &path(&trace), "--backend", "gpu"])
        .assert()
        .code(69);
}

#[test]
fn specs_lists_and_filters() {
    sentil()
        .args(["specs", "-o", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"verb\":\"specs\""));
    sentil()
        .args(["specs", "--filter", "aerospace"])
        .assert()
        .success()
        .stdout(predicate::str::contains("aerospace"));
}

#[test]
fn monitor_streams_and_summarizes() {
    sentil()
        .args(["monitor", "-f", "x > 0", "-o", "ndjson"])
        .write_stdin("{\"time\":0,\"x\":1}\n{\"time\":1,\"x\":-2}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"event\":\"sample\""))
        .stdout(predicate::str::contains("\"event\":\"summary\""));
}

#[test]
fn smc_estimates_a_probability() {
    let trace = file("time,x\n0,1\n1,1\n2,1\n");
    sentil()
        .args([
            "smc",
            "-f",
            "P>=0.9(always[0,2] (x > 0))",
            "-t",
            &path(&trace),
            "--samples",
            "500",
            "-o",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"verb\":\"smc\""))
        .stdout(predicate::str::contains("\"probability\""));
}

#[test]
fn synth_finds_a_feasible_input() {
    let model = file(
        "{\"a\":[[1.0]],\"b\":[[1.0]],\"x0\":[1.0],\"variables\":[\"x\"],\"dt\":1.0,\
         \"horizon\":3,\"bounds\":{\"lower\":[-1.0],\"upper\":[1.0]}}",
    );
    sentil()
        .args(["synth", "-f", "always (x > 0)", "--model", &path(&model), "-o", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"feasible\":true"));
}

#[test]
fn smc_accepts_noise_flags() {
    let trace = file("time,x\n0,1\n1,1\n2,1\n");
    sentil()
        .args([
            "smc",
            "-f",
            "P>=0.5(always[0,2] (x > 0))",
            "-t",
            &path(&trace),
            "--noise",
            "x=gaussian:0,2",
            "--samples",
            "400",
            "-o",
            "json",
        ])
        .assert()
        .stdout(predicate::str::contains("\"probability\""));
}

#[test]
fn lift_ensemble_tags_members() {
    let trace = file("time,x\n0,1\n1,1\n");
    sentil()
        .args(["lift", "--noise", "x=gaussian:0,0.3", "-t", &path(&trace), "--members", "3"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("member,time,x"))
        .stdout(predicate::str::contains("2,1,"));
}

#[test]
fn lift_accepts_extended_noise_families() {
    let trace = file("time,x\n0,1\n");
    sentil()
        .args(["lift", "--noise", "x=weibull:1.5,1", "-t", &path(&trace)])
        .assert()
        .success();
}

#[test]
fn lift_with_noise_needs_no_spec() {
    let trace = file("time,x\n0,1\n1,1\n");
    sentil()
        .args(["lift", "--noise", "x=gaussian:0,0.5", "-t", &path(&trace)])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("time,x"));
}

#[test]
fn bad_noise_distribution_errors() {
    let trace = file("time,x\n0,1\n");
    sentil()
        .args(["lift", "--noise", "x=nope:1,2", "-t", &path(&trace)])
        .assert()
        .code(65);
}

#[test]
fn check_reports_violation_intervals() {
    let trace = file("time,x\n0,5\n1,-3\n2,4\n3,-1\n");
    sentil()
        .args(["check", "-f", "x > 0", "-t", &path(&trace), "--violations", "--semantics", "discrete"])
        .assert()
        .code(10)
        .stdout(predicate::str::contains("[1.000, 1.000]"))
        .stdout(predicate::str::contains("[3.000, 3.000]"));
}

#[test]
fn smc_bayes_decides() {
    let trace = file("time,x\n0,1\n1,1\n2,1\n");
    sentil()
        .args(["smc", "-f", "P>=0.9(always[0,2] (x > 0))", "-t", &path(&trace), "--algo", "bayes", "-o", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"decision\":\"holds\""));
}

#[test]
fn maps_a_variable_to_a_column() {
    let trace = file("time,velocity\n0,5\n1,-2\n");
    sentil()
        .args([
            "check", "-f", "speed > 0", "-t", &path(&trace), "--map", "speed=velocity",
            "--semantics", "discrete", "--signal",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("5"));
}

#[test]
fn map_to_a_missing_column_errors() {
    let trace = file("time,velocity\n0,5\n");
    sentil()
        .args(["check", "-f", "speed > 0", "-t", &path(&trace), "--map", "speed=nope"])
        .assert()
        .code(65);
}

#[test]
fn reads_a_matlab_trace() {
    let mat = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sentil-core/tests/fixtures/sample_v5.mat"
    );
    sentil()
        .args(["check", "-f", "x > 0", "-t", mat, "--semantics", "discrete", "--signal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("10"));
}

#[test]
fn mine_finds_the_tightest_parameter() {
    let trace = file("time,output,reference\n0,1.0,1.0\n1,1.15,1.0\n2,1.0,1.0\n");
    sentil()
        .args(["mine", "--spec", "controls/overshoot", "--parameter", "max_overshoot", "-t", &path(&trace), "-o", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"tightest\""));
}

#[test]
fn lift_writes_csv() {
    let trace = file("time,response\n0,1.0\n1,1.05\n");
    sentil()
        .args(["lift", "--spec", "controls/overshoot", "-t", &path(&trace)])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("time,"));
}

#[test]
fn explain_operator_and_exit_codes() {
    sentil()
        .args(["explain", "until"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sup over"));
    sentil()
        .args(["explain", "exit-codes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("violated"));
}

#[test]
fn config_shows_the_search_paths() {
    sentil()
        .arg("config")
        .assert()
        .success()
        .stdout(predicate::str::contains("config files"));
}

#[test]
fn completion_and_man_emit() {
    sentil()
        .args(["completion", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_sentil"));
    sentil()
        .arg("man")
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH"));
}

#[test]
fn init_off_a_terminal_does_not_hang() {
    sentil().arg("init").write_stdin("").assert().code(65);
}

#[test]
fn runs_a_config_alias() {
    let config = file("[alias]\npos = \"check -f 'x > 0' --semantics discrete --signal\"\n");
    let trace = file("time,x\n0,5\n1,-2\n");
    sentil()
        .args(["--config", &path(&config), "pos", "-t", &path(&trace)])
        .assert()
        .success()
        .stdout(predicate::str::contains("5"));
}

#[test]
fn monitor_reports_a_violation_when_it_is_decided_not_when_the_window_closes() {
    sentil()
        .args(["monitor", "-f", "always[0,10] (speed < 30)", "-o", "ndjson"])
        .write_stdin("{\"time\":0,\"speed\":10}\n{\"time\":1,\"speed\":50}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""robustness":-20.0,"satisfied":false"#));
    sentil()
        .args(["monitor", "-f", "always[0,2] (x > 0)", "-o", "ndjson"])
        .write_stdin("{\"time\":0,\"x\":1}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""robustness":"nan""#));
}

#[test]
fn unknown_subcommand_errors() {
    sentil().arg("frobnicate").assert().code(65);
}

#[test]
fn no_arguments_shows_help() {
    sentil()
        .assert()
        .code(2)
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn version_carries_the_commit() {
    sentil()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("commit:"));
}