//! `sentil monitor` is the online monitor. One JSON sample per line in.

use std::collections::HashMap;
use std::io::{BufRead, IsTerminal, Write};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use sentil::{Formula, MultiFormulaMonitor, Robustness, SmcConfig};
use serde_json::json;

use crate::engine;
use crate::error::{code, CliError, Run};
use crate::interrupt;
use crate::output::{self, Out};

type Row = (String, Robustness, Option<f64>);

fn decided(robustness: Robustness) -> Option<f64> {
    match robustness {
        Robustness::Concrete(v) => Some(v),
        Robustness::Interval(_, upper) if upper < 0.0 => Some(upper),
        Robustness::Interval(lower, _) if lower >= 0.0 => Some(lower),
        Robustness::Interval(..) => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    formula: Option<&str>,
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    map: &[String],
    noise: &[String],
    particles: u64,
    out: &Out,
) -> Run {
    let mapping = engine::parse_map(map)?;
    let rename: HashMap<&str, &str> =
        mapping.iter().map(|(v, f)| (f.as_str(), v.as_str())).collect();
    let (combined, builder) = engine::resolve_formula(formula, spec, variant, params, false)?;
    let formulas: Vec<&str> = combined
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if formulas.is_empty() {
        return Err(CliError::Input("there is no formula to monitor".into(), None));
    }

    let ids: Vec<String> = (0..formulas.len()).map(|i| format!("f{i}")).collect();
    let mut monitor = build_bank(&formulas, &ids, noise, builder.as_ref(), particles)?;

    let interrupted = interrupt::flag();

    let live = out.is_text() && !out.quiet && std::io::stdout().is_terminal();
    let mut dashboard = live.then(|| Dashboard::new(formulas.iter().map(|s| s.to_string()).collect()));
    if out.is_text() && !out.quiet && !live {
        let banner = format!(
            "monitoring {} formula(s); send one JSON object per line, e.g. {{\"time\": 1.0, \"x\": 5.0}}",
            formulas.len()
        );
        eprintln!("{}", out.paint(&banner, output::dim()));
    }

    let reader = spawn_reader();
    let mut stdout = std::io::stdout();
    if dashboard.is_none() && !out.is_text() {
        let labels: serde_json::Map<String, serde_json::Value> = ids
            .iter()
            .zip(&formulas)
            .map(|(id, text)| (id.clone(), json!(*text)))
            .collect();
        let _ = writeln!(
            stdout,
            "{}",
            json!({ "schema_version": "1.0", "event": "formulas", "formulas": labels })
        );
    }
    let mut samples = 0u64;
    let mut skipped = 0u64;
    loop {
        if interrupted.load(Ordering::SeqCst) {
            break;
        }
        let line = match reader.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(line)) => line,
            Ok(Err(e)) => return Err(CliError::Input(format!("reading input: {e}"), None)),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (time, pairs) = match parse_sample(trimmed) {
            Ok(sample) => sample,
            Err(e) => {
                skipped += 1;
                if dashboard.is_none() {
                    warn(out, &format!("skipped a malformed sample: {e}"));
                }
                continue;
            }
        };
        let borrowed: Vec<(&str, f64)> = pairs
            .iter()
            .map(|(k, v)| (rename.get(k.as_str()).copied().unwrap_or(k.as_str()), *v))
            .collect();
        let results = match monitor.update(time, &borrowed) {
            Ok(results) => results,
            Err(e) => {
                skipped += 1;
                if dashboard.is_none() {
                    warn(out, &format!("skipped the sample at t={time}: {e}"));
                }
                continue;
            }
        };
        let rows: Vec<Row> = results
            .into_iter()
            .zip(monitor.probabilities())
            .map(|((id, robustness), (_, estimate))| (id, robustness, estimate))
            .collect();
        samples += 1;
        match dashboard.as_mut() {
            Some(board) => board.render(out, time, samples, skipped, &rows),
            None => {
                if !emit(&mut stdout, out, time, &rows)? {
                    break;
                }
            }
        }
    }

    if dashboard.is_none() && !out.is_text() {
        let _ = writeln!(
            stdout,
            "{}",
            json!({ "schema_version": "1.0", "event": "summary", "samples": samples })
        );
    }

    Ok(if interrupted.load(Ordering::SeqCst) {
        code::INTERRUPTED
    } else {
        code::SUCCESS
    })
}

fn build_bank(
    formulas: &[&str],
    ids: &[String],
    noise: &[String],
    builder: Option<&sentil::SpecBuilder>,
    particles: u64,
) -> Result<MultiFormulaMonitor, CliError> {
    let lifting = match engine::parse_noise(noise)? {
        Some(registry) => Some(registry),
        None => builder.and_then(|b| b.build_lifting_registry().ok()),
    };
    let config = SmcConfig {
        samples: particles,
        ..SmcConfig::default()
    };
    let mut monitor = MultiFormulaMonitor::new();
    for (id, text) in ids.iter().zip(formulas) {
        let parsed = engine::parse_or_diagnose(text)?;
        let added = if matches!(parsed, Formula::Probabilistic(..)) {
            let registry = lifting.as_ref().ok_or_else(|| {
                CliError::Input(
                    format!("'{text}' is probabilistic; give a noise model to monitor it online"),
                    Some("for example --noise 'speed=gaussian:0,0.5'".into()),
                )
            })?;
            monitor.add_probabilistic(id.clone(), &parsed, registry, &config)
        } else {
            monitor.add_formula(id.clone(), &parsed)
        };
        added.map_err(|e| CliError::Engine(e.to_string()))?;
    }
    Ok(monitor)
}

fn spawn_reader() -> mpsc::Receiver<std::io::Result<String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        let mut line = String::new();
        loop {
            line.clear();
            match handle.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let text = line.trim_end_matches(['\n', '\r']).to_string();
                    if tx.send(Ok(text)).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => break,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    });
    rx
}

struct Dashboard {
    labels: Vec<String>,
    height: usize,
    drawn: bool,
}

impl Dashboard {
    fn new(labels: Vec<String>) -> Self {
        Self {
            labels,
            height: 0,
            drawn: false,
        }
    }

    fn render(&mut self, out: &Out, time: f64, samples: u64, skipped: u64, rows: &[Row]) {
        let lines = frame(out, time, samples, skipped, &self.labels, rows);
        let mut buf = String::new();
        if self.drawn {
            // cursor back up over the previous box; each line is cleared as it is rewritten
            buf.push_str(&format!("\x1b[{}A", self.height));
        }
        for line in &lines {
            buf.push_str("\r\x1b[2K");
            buf.push_str(line);
            buf.push('\n');
        }
        self.height = lines.len();
        self.drawn = true;
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(buf.as_bytes());
        let _ = stdout.flush();
    }
}

/// box here. look at it before you change anything.
fn frame(
    out: &Out,
    time: f64,
    samples: u64,
    skipped: u64,
    labels: &[String],
    rows: &[Row],
) -> Vec<String> {
    const VALUE_W: usize = 13;
    let label_w = labels
        .iter()
        .map(|l| l.chars().count())
        .chain([7])
        .max()
        .unwrap_or(7)
        .min(44);
    let inner = label_w + 4 + VALUE_W; // " " + label + "  " + value + " "
    let rule = "─".repeat(inner);
    let bar = |left: char, right: char| out.paint(&format!("{left}{rule}{right}"), output::dim());
    let dim_bar = out.paint("│", output::dim());

    let title = " sentil monitor ";
    let title_fill = inner.saturating_sub(title.chars().count());
    let mut lines = vec![out.paint(
        &format!("┌{title}{}┐", "─".repeat(title_fill)),
        output::dim(),
    )];

    let meta = |label: &str, value: String| {
        format!("{dim_bar} {label:<label_w$}  {value:>VALUE_W$} {dim_bar}")
    };
    lines.push(meta("time", format!("{time:.2}")));
    lines.push(meta("samples", samples.to_string()));
    lines.push(meta("skipped", skipped.to_string()));
    lines.push(bar('├', '┤'));

    for (i, (_, robustness, estimate)) in rows.iter().enumerate() {
        let label = labels.get(i).map_or(String::new(), |l| truncate(l, label_w));
        let settled = decided(*robustness);
        let word = match settled {
            Some(v) if v >= 0.0 => out.paint(&format!("{:>4}", "sat"), output::good()),
            Some(_) => out.paint(&format!("{:>4}", "viol"), output::bad()),
            None => out.paint(&format!("{:>4}", "unk"), output::dim()),
        };
        let mark = if robustness.is_resolved() { ' ' } else { '~' };
        let cell = match (estimate, settled) {
            (Some(p), _) => format!("{word} {:>8}", format!("P={p:.3}")),
            (None, Some(v)) if v.is_finite() => format!("{word} {mark}{v:>7.3}"),
            (None, _) => format!("{word} {:>8}", ""),
        };
        lines.push(format!("{dim_bar} {label:<label_w$}  {cell} {dim_bar}"));
    }
    lines.push(bar('└', '┘'));
    lines.push(out.paint("  Ctrl+C to stop", output::dim()));
    lines
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

fn warn(out: &Out, message: &str) {
    if !out.quiet {
        eprintln!("{}", out.paint(message, output::dim()));
    }
}

fn parse_sample(line: &str) -> Result<(f64, Vec<(String, f64)>), CliError> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|e| CliError::Input(format!("invalid JSON sample: {e}"), None))?;
    let object = value
        .as_object()
        .ok_or_else(|| CliError::Input("each sample must be a JSON object".into(), None))?;
    let time = object
        .get("time")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| CliError::Input("the sample has no numeric 'time'".into(), None))?;
    let mut pairs = Vec::new();
    for (key, raw) in object {
        if key == "time" {
            continue;
        }
        if let Some(number) = raw.as_f64() {
            pairs.push((key.clone(), number));
        }
    }
    Ok((time, pairs))
}

fn emit(
    stdout: &mut impl Write,
    out: &Out,
    time: f64,
    rows: &[Row],
) -> Result<bool, CliError> {
    let write_result = if out.is_text() {
        let mut line = format!("[t={time:.3}]");
        for (id, robustness, estimate) in rows {
            let settled = decided(*robustness);
            let verdict = match settled {
                Some(v) if v >= 0.0 => out.paint("sat", output::good()),
                Some(_) => out.paint("viol", output::bad()),
                None => out.paint("unk", output::dim()),
            };
            let provisional = if robustness.is_resolved() { "" } else { "~" };
            match (estimate, settled) {
                (Some(p), _) => line.push_str(&format!("  {id} {verdict} P={p:.4}")),
                (None, Some(v)) if v.is_finite() => {
                    line.push_str(&format!("  {id} {verdict} {provisional}{v:.4}"));
                }
                (None, _) => line.push_str(&format!("  {id} {verdict}")),
            }
        }
        writeln!(stdout, "{line}")
    } else {
        let mut map = serde_json::Map::new();
        for (id, robustness, estimate) in rows {
            let settled = decided(*robustness);
            let magnitude = match settled {
                Some(v) if v.is_finite() => json!(v),
                Some(v) if v > 0.0 => json!("inf"),
                Some(_) => json!("-inf"),
                None => json!("nan"),
            };
            let mut record = json!({
                "robustness": magnitude,
                "satisfied": settled.is_some_and(|v| v >= 0.0),
                "resolved": robustness.is_resolved(),
            });
            if let Some(p) = estimate {
                record["probability"] = json!(p);
            }
            map.insert(id.clone(), record);
        }
        writeln!(
            stdout,
            "{}",
            json!({ "schema_version": "1.0", "event": "sample", "time": time, "results": map })
        )
    };
    match write_result {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(false),
        Err(e) => Err(CliError::Internal(format!("writing output: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ColorWhen, OutputFormat};

    fn plain() -> Out {
        Out::new(OutputFormat::Text, false, ColorWhen::Never, false, false)
    }

    #[test]
    fn the_box_is_rectangular() {
        let out = plain();
        let labels = vec!["always (x > 0)".to_string(), "historically (y < 9)".to_string()];
        let rows: Vec<Row> = vec![
            ("f0".to_string(), Robustness::Concrete(0.5), None),
            ("f1".to_string(), Robustness::Concrete(-2.0), Some(0.83)),
        ];
        let lines = frame(&out, 1.5, 42, 0, &labels, &rows);
        // top, three meta rows, separator, two formula rows, bottom, then the caption.
        assert_eq!(lines.len(), 9);
        let box_lines = &lines[..lines.len() - 1];
        let width = box_lines[0].chars().count();
        for line in box_lines {
            assert_eq!(line.chars().count(), width, "ragged line: {line}");
        }
        assert!(lines.iter().any(|l| l.contains("always (x > 0)")));
        assert!(lines.iter().any(|l| l.contains("sat")));
        assert!(lines.iter().any(|l| l.contains("viol")));
        assert!(lines.iter().any(|l| l.contains("P=0.830")));
        assert!(lines.last().unwrap().contains("Ctrl+C to stop"));
    }

    #[test]
    fn an_undetermined_verdict_reads_as_unknown_not_violated() {
        let ndjson = Out::new(OutputFormat::Ndjson, false, ColorWhen::Never, false, false);
        for (formula, settles_at) in [("always[0, 2](always[0, 1](x > 0))", 3), ("always[0, 2](x > 0)", 2)] {
            let mut monitor = sentil::StreamMonitor::new(formula).unwrap();
            let slot = monitor.symbol_index("x").unwrap();
            let mut packed = vec![0.0; monitor.variable_count()];
            for step in 0..5 {
                packed[slot] = 1.0;
                let robustness = monitor.update_packed(f64::from(step), &packed).unwrap();
                let rows: Vec<Row> = vec![("f0".to_string(), robustness, None)];
                let row = frame(&plain(), f64::from(step), 1, 0, &[formula.to_string()], &rows)
                    .into_iter()
                    .find(|l| l.contains(formula))
                    .unwrap();
                let mut buf: Vec<u8> = Vec::new();
                emit(&mut buf, &ndjson, f64::from(step), &rows).unwrap();
                let record: serde_json::Value = serde_json::from_slice(&buf).unwrap();
                let resolved = record["results"]["f0"]["resolved"].as_bool().unwrap();

                assert_eq!(resolved, step >= settles_at, "{formula} at {step}");
                if resolved {
                    assert!(row.contains("sat"), "{formula} at {step}: {row}");
                } else {
                    assert!(row.contains("unk"), "{formula} at {step}: {row}");
                    assert!(!row.contains("viol"), "unresolved shown as a violation: {row}");
                    assert!(!row.contains("NaN"), "NaN reached the display: {row}");
                }
            }
        }
    }

    #[test]
    fn an_unknown_margin_serializes_as_nan() {
        let rows: Vec<Row> = vec![("f0".to_string(), Robustness::UNKNOWN, None)];
        let mut buf: Vec<u8> = Vec::new();
        let out = Out::new(OutputFormat::Ndjson, false, ColorWhen::Never, false, false);
        emit(&mut buf, &out, 0.0, &rows).unwrap();
        let record: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        let f0 = &record["results"]["f0"];
        assert_eq!(f0["robustness"], "nan");
        assert_eq!(f0["resolved"], false);
        assert_eq!(f0["satisfied"], false);
    }
}