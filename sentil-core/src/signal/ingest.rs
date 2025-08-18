//! Reading traces from delimited text, and from the columnar and database formats.
//!
//! [`Trace::from_csv_str`] reads comma-separated data with a header row: the
//! time column is found by name (`time`, `timestamp`, and so on) or falls back
//! to the first column, and every other column becomes a signal. File loading
//! and the columnar and database formats build on this foundation.

use std::path::Path;

use crate::error::{Error, Result};
use crate::signal::Trace;

/// Names taken for the time axis, in priority order.
const TIME_FIELD_CANDIDATES: &[&str] = &[
    "time",
    "timestamp",
    "t",
    "time_s",
    "time_sec",
    "time_ms",
    "time_ns",
    "elapsed",
    "elapsed_time",
    "epoch",
];

impl Trace {
    /// Reads a trace from comma-separated text with a header row.
    ///
    /// ```
    /// use sentil::Trace;
    ///
    /// let trace = Trace::from_csv_str("time,x,y\n0,10,1\n1,5,2\n2,1,3")?;
    /// assert_eq!(trace.len(), 3);
    /// assert_eq!(trace.variables(), vec!["x", "y"]);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ingest`] if the text has no columns or a cell is not a number.
    pub fn from_csv_str(text: &str) -> Result<Self> {
        read_delimited(text, b',')
    }

    /// Reads a trace from tab-separated text with a header row.
    ///
    /// ```
    /// use sentil::Trace;
    ///
    /// let trace = Trace::from_tsv_str("time\tx\n0\t10\n1\t5")?;
    /// assert_eq!(trace.variables(), vec!["x"]);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ingest`] on the same conditions as [`Trace::from_csv_str`].
    pub fn from_tsv_str(text: &str) -> Result<Self> {
        read_delimited(text, b'\t')
    }

    /// Reads a trace from a file, choosing the reader from the extension.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Ingest`] if the file cannot be read, the extension is
    /// unrecognized, or the matching feature is off.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match extension(path).as_str() {
            "csv" | "txt" => from_delimited_file(path, b','),
            "tsv" => from_delimited_file(path, b'\t'),
            other => Err(ingest_at(
                path,
                format!("unrecognized file extension '{other}'"),
            )),
        }
    }
}

fn read_delimited(text: &str, delimiter: u8) -> Result<Trace> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .from_reader(text.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| ingest(None, e.to_string()))?
        .iter()
        .map(str::to_owned)
        .collect();
    if headers.is_empty() {
        return Err(ingest(None, "the input has no columns"));
    }
    let time_idx = detect_time_column(&headers).unwrap_or(0);
    let signals: Vec<(String, usize)> = headers
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != time_idx)
        .map(|(i, h)| (h.clone(), i))
        .collect();

    let mut times = Vec::new();
    let mut columns: Vec<Vec<f64>> = vec![Vec::new(); signals.len()];
    for (r, record) in reader.records().enumerate() {
        let row = r + 2;
        let record = record.map_err(|e| ingest(Some(row), e.to_string()))?;
        times.push(parse_cell(&record, time_idx, row, "time")?);
        for (slot, &(_, idx)) in signals.iter().enumerate() {
            columns[slot].push(parse_cell(&record, idx, row, &headers[idx])?);
        }
    }

    let named = signals
        .into_iter()
        .zip(columns)
        .map(|((name, _), column)| (name, column))
        .collect();
    assemble(times, named)
}

fn assemble(time: Vec<f64>, signals: Vec<(String, Vec<f64>)>) -> Result<Trace> {
    let mut trace = Trace::new(time).map_err(|e| ingest(None, e.to_string()))?;
    for (name, column) in signals {
        trace
            .add_signal(&name, column)
            .map_err(|e| ingest(None, e.to_string()))?;
    }
    Ok(trace)
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn from_delimited_file(path: &Path, delimiter: u8) -> Result<Trace> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| ingest_at(path, format!("could not open the file: {e}")))?;
    read_delimited(&text, delimiter).map_err(|e| with_path(e, path))
}

fn ingest_at(path: &Path, message: impl Into<String>) -> Error {
    Error::Ingest {
        path: Some(path.display().to_string()),
        row: None,
        message: message.into(),
    }
}

fn with_path(err: Error, path: &Path) -> Error {
    match err {
        Error::Ingest { row, message, .. } => Error::Ingest {
            path: Some(path.display().to_string()),
            row,
            message,
        },
        other => other,
    }
}

fn detect_time_column(headers: &[String]) -> Option<usize> {
    let normalized: Vec<String> = headers.iter().map(|h| h.trim().to_lowercase()).collect();
    TIME_FIELD_CANDIDATES
        .iter()
        .find_map(|&candidate| normalized.iter().position(|h| h == candidate))
}

fn parse_cell(record: &csv::StringRecord, idx: usize, row: usize, what: &str) -> Result<f64> {
    let cell = record
        .get(idx)
        .ok_or_else(|| ingest(Some(row), format!("missing column {idx}")))?;
    cell.trim().parse::<f64>().map_err(|_| {
        ingest(
            Some(row),
            format!("could not parse {what} value '{}'", cell.trim()),
        )
    })
}

fn ingest(row: Option<usize>, message: impl Into<String>) -> Error {
    let message = message.into();
    let located = match row {
        Some(r) => format!("row {r}: {message}"),
        None => message,
    };
    Error::Ingest {
        path: None,
        row,
        message: located,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_csv_file_by_path() {
        let mut path = std::env::temp_dir();
        path.push(format!("sentil_ingest_{}.csv", std::process::id()));
        std::fs::write(&path, "time,x\n0,10\n1,5\n2,1").unwrap();
        let trace = Trace::from_path(&path);
        std::fs::remove_file(&path).ok();
        let trace = trace.unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.variables(), vec!["x"]);
    }

    #[test]
    fn an_unrecognized_extension_is_rejected() {
        assert!(Trace::from_path("/tmp/whatever.xyz").is_err());
    }
}