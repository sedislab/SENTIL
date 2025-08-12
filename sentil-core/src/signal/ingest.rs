//! Reading traces from delimited text, and from the columnar and database formats.
//!
//! [`Trace::from_csv_str`] reads comma-separated data with a header row: the
//! time column is found by name (`time`, `timestamp`, and so on) or falls back
//! to the first column, and every other column becomes a signal. File loading
//! and the columnar and database formats build on this foundation.

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

    let mut trace = Trace::new(times).map_err(|e| ingest(None, e.to_string()))?;
    for ((name, _), column) in signals.iter().zip(columns) {
        trace
            .add_signal(name, column)
            .map_err(|e| ingest(None, e.to_string()))?;
    }
    Ok(trace)
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