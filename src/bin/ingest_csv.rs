//! Streaming CSV-to-ClickHouse ingestion binary for telecom network alarm exports.

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use braincode_be::utils::validation::redact_dsn;
use chrono::DateTime;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Number of rows flushed per HTTP `JSONEachRow` ClickHouse insert request.
const BATCH_SIZE: usize = 15_000;

/// Buffered file reader capacity (8 MiB) for streaming large CSV exports efficiently.
const READER_BUFFER_BYTES: usize = 8 * 1024 * 1024;

/// HTTP client request timeout in seconds for ClickHouse batch inserts.
const HTTP_TIMEOUT_SECS: u64 = 60;

/// Expected row count for default Telkomsel FBB alarm export progress display.
const DEFAULT_PROGRESS_LEN: u64 = 194_575;

/// Fallback timestamp used when `alarm_start` cannot be parsed from CSV input.
const FALLBACK_ALARM_DATETIME: &str = "2026-09-14 00:00:00";

/// Serialized row structure matching the ClickHouse `active_alarms` table schema.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClickHouseRecord {
    pub identifier: String,
    pub ticketid: Option<String>,
    pub alarmid: String,
    pub alarmserialnumber: String,
    pub cleartime: i64,
    pub severity: u8,
    pub ticketstatus: i32,
    pub summary: String,
    pub alarmname: String,
    pub alarmtext: String,
    pub alarmtype: String,
    pub domain: i32,
    pub location: String,
    pub area: String,
    pub sitecode: String,
    pub siteid: String,
    pub sitename: String,
    pub node: String,
    pub devicename: String,
    pub ipaddress: String,
    pub vendor: String,
    pub firstoccurrence: i64,
    pub lastoccurrence: i64,
    pub alarm_start: String,
    pub acknowledged: String,
    pub totalcount: u32,
    pub eventtype: String,
    pub raw_attributes: String,
}

/// Parses an integer epoch timestamp, defaulting to `0` on empty or malformed input.
fn parse_epoch_or_zero(value: &str) -> i64 {
    value.trim().parse::<i64>().unwrap_or(0)
}

/// Normalizes an ISO datetime string or Unix millisecond epoch into `YYYY-MM-DD HH:MM:SS`.
fn parse_datetime_string(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 19 && trimmed.is_char_boundary(19) {
        return trimmed[0..19].to_string();
    }

    if let Ok(epoch_ms) = trimmed.parse::<i64>() {
        let secs = epoch_ms / 1000;
        if let Some(dt) = DateTime::from_timestamp(secs, 0) {
            return dt.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }

    FALLBACK_ALARM_DATETIME.to_string()
}

/// Resolves the input CSV path from CLI arguments or known default filenames.
fn resolve_csv_path(args: &[String]) -> String {
    if let Some(explicit) = args.get(1) {
        return explicit.clone();
    }

    let default_paths = [
        "alarm_fbb_202609141305.csv",
        "../alarm_fbb_202609141305.csv",
        "alarm_fbb_sample_10_rows.csv",
        "../alarm_fbb_sample_10_rows.csv",
    ];

    default_paths
        .iter()
        .find(|path| Path::new(path).exists())
        .map(|path| (*path).to_string())
        .unwrap_or_else(|| "alarm_fbb_202609141305.csv".to_string())
}

/// Looks up a trimmed column value from a CSV record using the normalized header map.
fn get_column(record: &csv::StringRecord, col_map: &HashMap<String, usize>, name: &str) -> String {
    col_map
        .get(name)
        .and_then(|&idx| record.get(idx))
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Serializes all non-empty columns into a single JSON object to preserve raw CSV attributes.
fn build_raw_attributes(headers: &csv::StringRecord, record: &csv::StringRecord) -> String {
    let mut raw_map = serde_json::Map::with_capacity(headers.len());
    for (idx, field_name) in headers.iter().enumerate() {
        if let Some(val) = record.get(idx).filter(|v| !v.is_empty()) {
            raw_map.insert(
                field_name.to_string(),
                serde_json::Value::String(val.to_string()),
            );
        }
    }
    serde_json::Value::Object(raw_map).to_string()
}

/// Maps a single CSV record into a [`ClickHouseRecord`], returning `None` if `identifier` is blank.
fn map_csv_record(
    headers: &csv::StringRecord,
    col_map: &HashMap<String, usize>,
    record: &csv::StringRecord,
) -> Option<ClickHouseRecord> {
    let identifier = get_column(record, col_map, "identifier");
    if identifier.is_empty() {
        return None;
    }

    let ticket_raw = get_column(record, col_map, "ticketid");
    let ticketid = (!ticket_raw.is_empty()).then_some(ticket_raw);
    let devicename = get_column(record, col_map, "devicename");
    let node_raw = get_column(record, col_map, "node");
    let node = if node_raw.is_empty() {
        devicename.clone()
    } else {
        node_raw
    };

    let siteid = get_column(record, col_map, "siteid");
    let sitecode_raw = get_column(record, col_map, "sitecode");
    let sitecode = if sitecode_raw.is_empty() {
        siteid.clone()
    } else {
        sitecode_raw
    };

    Some(ClickHouseRecord {
        identifier,
        ticketid,
        alarmid: get_column(record, col_map, "alarmid"),
        alarmserialnumber: get_column(record, col_map, "alarmserialnumber"),
        cleartime: parse_epoch_or_zero(&get_column(record, col_map, "cleartime")),
        severity: get_column(record, col_map, "severity").parse().unwrap_or(1),
        ticketstatus: get_column(record, col_map, "ticketstatus")
            .parse()
            .unwrap_or(0),
        summary: get_column(record, col_map, "summary"),
        alarmname: get_column(record, col_map, "alarmname"),
        alarmtext: get_column(record, col_map, "alarmtext"),
        alarmtype: get_column(record, col_map, "alarmtype"),
        domain: get_column(record, col_map, "domain").parse().unwrap_or(0),
        location: get_column(record, col_map, "location"),
        area: get_column(record, col_map, "area"),
        sitecode,
        siteid,
        sitename: get_column(record, col_map, "sitename"),
        node,
        devicename,
        ipaddress: get_column(record, col_map, "ipaddress"),
        vendor: get_column(record, col_map, "vendor"),
        firstoccurrence: parse_epoch_or_zero(&get_column(record, col_map, "firstoccurrence")),
        lastoccurrence: parse_epoch_or_zero(&get_column(record, col_map, "lastoccurrence")),
        alarm_start: parse_datetime_string(&get_column(record, col_map, "alarm_start")),
        acknowledged: get_column(record, col_map, "acknowledged"),
        totalcount: get_column(record, col_map, "totalcount")
            .parse()
            .unwrap_or(1),
        eventtype: get_column(record, col_map, "eventtype"),
        raw_attributes: build_raw_attributes(headers, record),
    })
}

/// Streams records from the CSV reader and pushes batches to ClickHouse.
async fn ingest_csv_stream(
    reader: &mut csv::Reader<BufReader<File>>,
    client: &reqwest::Client,
    insert_url: &str,
) -> Result<(usize, usize)> {
    let headers = reader.headers()?.clone();
    let col_map: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.trim().to_lowercase(), idx))
        .collect();

    tracing::info!(header_columns = headers.len(), "Parsed CSV header schema");

    let pb = ProgressBar::new(DEFAULT_PROGRESS_LEN);
    let style = ProgressStyle::default_bar()
        .template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | {per_sec} | ETA: {eta}",
        )
        .context("Failed to configure progress bar template")?;
    pb.set_style(style);

    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut total_rows = 0usize;
    let mut skipped_rows = 0usize;

    for result in reader.records() {
        let Ok(record) = result else {
            skipped_rows += 1;
            continue;
        };

        let Some(item) = map_csv_record(&headers, &col_map, &record) else {
            skipped_rows += 1;
            continue;
        };

        batch.push(item);
        total_rows += 1;
        pb.inc(1);

        if batch.len() >= BATCH_SIZE {
            send_batch(client, insert_url, &batch).await?;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        send_batch(client, insert_url, &batch).await?;
    }

    pb.finish_with_message("Ingestion completed");
    Ok((total_rows, skipped_rows))
}

/// Sends a batch of [`ClickHouseRecord`] items using ClickHouse's `JSONEachRow` HTTP endpoint.
async fn send_batch(client: &reqwest::Client, url: &str, batch: &[ClickHouseRecord]) -> Result<()> {
    let mut payload = String::with_capacity(batch.len() * 512);
    for item in batch {
        payload.push_str(&serde_json::to_string(item)?);
        payload.push('\n');
    }

    let resp = client.post(url).body(payload).send().await?;
    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("ClickHouse insert batch error: {err_text}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ingest_csv=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args: Vec<String> = env::args().collect();
    let csv_file_path = resolve_csv_path(&args);

    let file = File::open(&csv_file_path)
        .with_context(|| format!("Gagal membuka file CSV di path '{csv_file_path}'"))?;
    let file_size_mb = file.metadata()?.len() as f64 / (1024.0 * 1024.0);

    let ch_url = env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".to_string());
    let ch_db = env::var("CLICKHOUSE_DB").unwrap_or_else(|_| "telkomsel".to_string());
    let redacted_ch_url = redact_dsn(&ch_url);

    tracing::info!(
        csv_path = %csv_file_path,
        file_size_mb = format!("{file_size_mb:.2}"),
        clickhouse_url = %redacted_ch_url,
        database = %ch_db,
        "Starting CSV to ClickHouse ingestion"
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?;

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .flexible(true)
        .from_reader(BufReader::with_capacity(READER_BUFFER_BYTES, file));

    let insert_url = format!(
        "{}/?query=INSERT%20INTO%20{}.active_alarms%20FORMAT%20JSONEachRow",
        ch_url.trim_end_matches('/'),
        ch_db
    );

    let start_time = Instant::now();
    let (total_rows, skipped_rows) = ingest_csv_stream(&mut reader, &client, &insert_url).await?;
    let elapsed_secs = start_time.elapsed().as_secs_f64();
    let throughput = total_rows as f64 / elapsed_secs.max(0.001);

    tracing::info!(
        total_rows,
        skipped_rows,
        elapsed_secs = format!("{elapsed_secs:.2}"),
        rows_per_sec = format!("{throughput:.0}"),
        "ClickHouse CSV ingestion finished"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_epoch_or_zero_handles_valid_and_invalid_values() {
        assert_eq!(parse_epoch_or_zero("1757308934000"), 1_757_308_934_000);
        assert_eq!(parse_epoch_or_zero("  0 "), 0);
        assert_eq!(parse_epoch_or_zero("not-a-number"), 0);
    }

    #[test]
    fn parse_datetime_string_handles_iso_epoch_and_fallback() {
        assert_eq!(
            parse_datetime_string("2026-09-14 13:05:00.123"),
            "2026-09-14 13:05:00"
        );
        assert_eq!(parse_datetime_string("0"), "1970-01-01 00:00:00");
        assert_eq!(parse_datetime_string("invalid"), FALLBACK_ALARM_DATETIME);
    }
}
