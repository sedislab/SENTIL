//! Reading traces from delimited text, and from the columnar and database formats.
//!
//! Times are read in the units the source carries.

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
            "parquet" | "pq" => load_parquet(path),
            "arrow" | "feather" | "ipc" => load_arrow(path),
            "db" | "sqlite" | "sqlite3" => load_sqlite(path),
            "h5" | "hdf5" | "mat" => load_hdf5(path),
            "mcap" => load_mcap(path),
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

/// Picks the time column from named f64 columns and assembles the trace, the
/// shared tail of the columnar and database readers.
#[cfg(any(
    feature = "parquet",
    feature = "arrow",
    feature = "sqlite",
    feature = "hdf5"
))]
fn from_columns(columns: Vec<(String, Vec<f64>)>, path: &Path) -> Result<Trace> {
    if columns.is_empty() {
        return Err(ingest_at(path, "the file has no numeric columns"));
    }
    let names: Vec<String> = columns.iter().map(|(n, _)| n.clone()).collect();
    let mut signals = columns;
    let time = signals.remove(detect_time_column(&names).unwrap_or(0)).1;
    assemble(time, signals).map_err(|e| with_path(e, path))
}

/// Reads each Arrow column as f64 through Arrow's cast kernels, dropping any
/// column that is not numeric. A null cell becomes NaN.
#[cfg(any(feature = "parquet", feature = "arrow"))]
fn arrow_columns(batches: &[arrow::array::RecordBatch]) -> Vec<(String, Vec<f64>)> {
    use arrow::array::Float64Array;
    use arrow::datatypes::DataType;

    let Some(first) = batches.first() else {
        return Vec::new();
    };
    let schema = first.schema();
    let mut columns: Vec<Vec<f64>> = vec![Vec::new(); schema.fields().len()];
    let mut numeric = vec![true; columns.len()];
    for batch in batches {
        for (i, column) in batch.columns().iter().enumerate() {
            match arrow::compute::cast(column, &DataType::Float64) {
                Ok(cast) => {
                    let values = cast
                        .as_any()
                        .downcast_ref::<Float64Array>()
                        .expect("a cast to Float64 yields a Float64Array");
                    columns[i].extend(values.iter().map(|v| v.unwrap_or(f64::NAN)));
                }
                Err(_) => numeric[i] = false,
            }
        }
    }
    schema
        .fields()
        .iter()
        .zip(columns)
        .zip(numeric)
        .filter(|(_, keep)| *keep)
        .map(|((field, column), _)| (field.name().clone(), column))
        .collect()
}

#[cfg(feature = "parquet")]
fn load_parquet(path: &Path) -> Result<Trace> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let file = std::fs::File::open(path)
        .map_err(|e| ingest_at(path, format!("could not open the file: {e}")))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| ingest_at(path, format!("not a valid Parquet file: {e}")))?
        .build()
        .map_err(|e| ingest_at(path, e.to_string()))?;
    let mut batches = Vec::new();
    for batch in reader {
        batches.push(batch.map_err(|e| ingest_at(path, e.to_string()))?);
    }
    from_columns(arrow_columns(&batches), path)
}

#[cfg(not(feature = "parquet"))]
fn load_parquet(path: &Path) -> Result<Trace> {
    Err(ingest_at(
        path,
        "Parquet files need the `parquet` feature enabled",
    ))
}

#[cfg(feature = "arrow")]
fn load_arrow(path: &Path) -> Result<Trace> {
    use arrow::ipc::reader::FileReader;

    let file = std::fs::File::open(path)
        .map_err(|e| ingest_at(path, format!("could not open the file: {e}")))?;
    let reader = FileReader::try_new(file, None)
        .map_err(|e| ingest_at(path, format!("not a valid Arrow IPC file: {e}")))?;
    let mut batches = Vec::new();
    for batch in reader {
        batches.push(batch.map_err(|e| ingest_at(path, e.to_string()))?);
    }
    from_columns(arrow_columns(&batches), path)
}

#[cfg(not(feature = "arrow"))]
fn load_arrow(path: &Path) -> Result<Trace> {
    Err(ingest_at(
        path,
        "Arrow IPC files need the `arrow` feature enabled",
    ))
}

/// Reads a trace from the first table of a SQLite database.
#[cfg(feature = "sqlite")]
fn load_sqlite(path: &Path) -> Result<Trace> {
    use rusqlite::{Connection, OpenFlags};

    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| ingest_at(path, format!("could not open the database: {e}")))?;
    let table: String = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| ingest_at(path, format!("no table to read: {e}")))?;
    let columns = read_sqlite_table(&conn, &table).map_err(|e| with_path(e, path))?;
    from_columns(columns, path)
}

/// Reads every numeric column of a SQLite table as f64; text and blob columns are
/// dropped, a null cell becomes NaN.
#[cfg(feature = "sqlite")]
#[allow(
    clippy::cast_precision_loss,
    reason = "integer columns are counts or timestamps that fit f64 exactly to 2^53"
)]
fn read_sqlite_table(conn: &rusqlite::Connection, table: &str) -> Result<Vec<(String, Vec<f64>)>> {
    use rusqlite::types::ValueRef;

    let mut stmt = conn
        .prepare(&format!("SELECT * FROM \"{table}\""))
        .map_err(|e| ingest(None, format!("could not read table '{table}': {e}")))?;
    let names: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|n| (*n).to_owned())
        .collect();
    let mut columns: Vec<Vec<f64>> = vec![Vec::new(); names.len()];
    let mut numeric = vec![true; names.len()];
    let mut rows = stmt.query([]).map_err(|e| ingest(None, e.to_string()))?;
    while let Some(row) = rows.next().map_err(|e| ingest(None, e.to_string()))? {
        for (i, column) in columns.iter_mut().enumerate() {
            match row.get_ref(i).map_err(|e| ingest(None, e.to_string()))? {
                ValueRef::Integer(v) => column.push(v as f64),
                ValueRef::Real(v) => column.push(v),
                ValueRef::Null => column.push(f64::NAN),
                ValueRef::Text(_) | ValueRef::Blob(_) => numeric[i] = false,
            }
        }
    }
    Ok(keep_numeric(names, columns, &numeric))
}

#[cfg(not(feature = "sqlite"))]
fn load_sqlite(path: &Path) -> Result<Trace> {
    Err(ingest_at(
        path,
        "SQLite databases need the `sqlite` feature enabled",
    ))
}

#[cfg(feature = "hdf5")]
fn load_hdf5(path: &Path) -> Result<Trace> {
    let file = hdf5::File::open(path)
        .map_err(|e| ingest_at(path, format!("could not open the file: {e}")))?;
    let names = file
        .member_names()
        .map_err(|e| ingest_at(path, e.to_string()))?;
    let mut columns = Vec::new();
    for name in names {
        if let Ok(dataset) = file.dataset(&name) {
            if let Ok(values) = dataset.read_raw::<f64>() {
                columns.push((name, values));
            }
        }
    }
    from_columns(columns, path)
}

#[cfg(not(feature = "hdf5"))]
fn load_hdf5(path: &Path) -> Result<Trace> {
    Err(ingest_at(
        path,
        "HDF5 files need the `hdf5` feature enabled",
    ))
}

/// Reads a trace from the JSON-encoded messages of an MCAP recording. Each
/// message's log time is the sample time; its numeric fields become signals,
/// aligned across messages with a missing field read as NaN.
#[cfg(feature = "mcap")]
#[allow(
    clippy::cast_precision_loss,
    reason = "nanosecond log times fit f64 over any realistic recording horizon"
)]
fn load_mcap(path: &Path) -> Result<Trace> {
    let bytes = std::fs::read(path)
        .map_err(|e| ingest_at(path, format!("could not open the file: {e}")))?;
    let stream = mcap::MessageStream::new(&bytes)
        .map_err(|e| ingest_at(path, format!("not a valid MCAP file: {e}")))?;
    let mut records: Vec<(f64, serde_json::Map<String, serde_json::Value>)> = Vec::new();
    for message in stream {
        let message = message.map_err(|e| ingest_at(path, e.to_string()))?;
        if let Ok(serde_json::Value::Object(obj)) = serde_json::from_slice(&message.data) {
            records.push((message.log_time as f64 / 1e9, obj));
        }
    }
    if records.is_empty() {
        return Err(ingest_at(path, "no JSON-encoded messages to read"));
    }
    records.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut fields: Vec<String> = Vec::new();
    for (_, obj) in &records {
        for (key, value) in obj {
            if value.is_number() && !fields.contains(key) {
                fields.push(key.clone());
            }
        }
    }
    let times: Vec<f64> = records.iter().map(|(t, _)| *t).collect();
    let signals = fields
        .into_iter()
        .map(|name| {
            let column = records
                .iter()
                .map(|(_, obj)| {
                    obj.get(&name)
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(f64::NAN)
                })
                .collect();
            (name, column)
        })
        .collect();
    assemble(times, signals).map_err(|e| with_path(e, path))
}

#[cfg(not(feature = "mcap"))]
fn load_mcap(path: &Path) -> Result<Trace> {
    Err(ingest_at(
        path,
        "MCAP files need the `mcap` feature enabled",
    ))
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

    #[cfg(feature = "parquet")]
    #[test]
    fn reads_a_parquet_file_by_path() {
        use arrow::array::{Float64Array, RecordBatch};
        use parquet::arrow::ArrowWriter;
        use std::sync::Arc;

        let batch = RecordBatch::try_from_iter(vec![
            (
                "time",
                Arc::new(Float64Array::from(vec![0.0, 1.0, 2.0])) as _,
            ),
            ("x", Arc::new(Float64Array::from(vec![10.0, 5.0, 1.0])) as _),
        ])
        .unwrap();
        let mut path = std::env::temp_dir();
        path.push(format!("sentil_ingest_{}.parquet", std::process::id()));
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = ArrowWriter::try_new(file, batch.schema(), None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        let trace = Trace::from_path(&path);
        std::fs::remove_file(&path).ok();
        let trace = trace.unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.variables(), vec!["x"]);
    }

    #[cfg(feature = "arrow")]
    #[test]
    fn reads_an_arrow_ipc_file_by_path() {
        use arrow::array::{Float64Array, RecordBatch};
        use arrow::ipc::writer::FileWriter;
        use std::sync::Arc;

        let batch = RecordBatch::try_from_iter(vec![
            (
                "time",
                Arc::new(Float64Array::from(vec![0.0, 1.0, 2.0])) as _,
            ),
            ("x", Arc::new(Float64Array::from(vec![10.0, 5.0, 1.0])) as _),
        ])
        .unwrap();
        let mut path = std::env::temp_dir();
        path.push(format!("sentil_ingest_{}.arrow", std::process::id()));
        let schema = batch.schema();
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = FileWriter::try_new(file, &schema).unwrap();
        writer.write(&batch).unwrap();
        writer.finish().unwrap();
        let trace = Trace::from_path(&path);
        std::fs::remove_file(&path).ok();
        let trace = trace.unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.variables(), vec!["x"]);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn reads_a_sqlite_table_by_path() {
        let mut path = std::env::temp_dir();
        path.push(format!("sentil_ingest_{}.sqlite", std::process::id()));
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute("CREATE TABLE trace (time REAL, x REAL)", [])
                .unwrap();
            conn.execute("INSERT INTO trace VALUES (0, 10), (1, 5), (2, 1)", [])
                .unwrap();
        }
        let trace = Trace::from_path(&path);
        std::fs::remove_file(&path).ok();
        let trace = trace.unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.variables(), vec!["x"]);
    }

    #[cfg(feature = "hdf5")]
    #[test]
    fn reads_an_hdf5_file_by_path() {
        let mut path = std::env::temp_dir();
        path.push(format!("sentil_ingest_{}.h5", std::process::id()));
        {
            let file = hdf5::File::create(&path).unwrap();
            file.new_dataset_builder()
                .with_data(&[0.0_f64, 1.0, 2.0])
                .create("time")
                .unwrap();
            file.new_dataset_builder()
                .with_data(&[10.0_f64, 5.0, 1.0])
                .create("x")
                .unwrap();
        }
        let trace = Trace::from_path(&path);
        std::fs::remove_file(&path).ok();
        let trace = trace.unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.variables(), vec!["x"]);
    }

    #[cfg(feature = "mcap")]
    #[test]
    fn reads_an_mcap_file_by_path() {
        use std::borrow::Cow;
        use std::sync::Arc;

        let mut buf = Vec::new();
        {
            let mut writer = mcap::Writer::new(std::io::Cursor::new(&mut buf)).unwrap();
            let channel = Arc::new(mcap::Channel {
                id: 0,
                topic: "/x".to_owned(),
                schema: None,
                message_encoding: "json".to_owned(),
                metadata: std::collections::BTreeMap::new(),
            });
            for (sequence, x) in (0u32..).zip([10.0_f64, 5.0, 1.0]) {
                let log_time = u64::from(sequence) * 1_000_000_000;
                let data = format!("{{\"x\":{x}}}");
                writer
                    .write(&mcap::Message {
                        channel: channel.clone(),
                        sequence,
                        log_time,
                        publish_time: log_time,
                        data: Cow::Owned(data.into_bytes()),
                    })
                    .unwrap();
            }
            writer.finish().unwrap();
        }
        let mut path = std::env::temp_dir();
        path.push(format!("sentil_ingest_{}.mcap", std::process::id()));
        std::fs::write(&path, &buf).unwrap();
        let trace = Trace::from_path(&path);
        std::fs::remove_file(&path).ok();
        let trace = trace.unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.variables(), vec!["x"]);
    }
}