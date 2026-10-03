//! The `botspy scan` verb: mine the adapters into the local store.
//!
//! One library call — the store's CDC pass, with pruning disabled so an
//! adapter that reports nothing can never delete stored history — plus
//! flags, exit codes, and rendering. On an empty store the pass is the
//! full cold ingest; on a warm store it touches only what changed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::Args;
use serde_json::Value;

use super::{
    check_root, home_of, real_source, render, selected_sources, valid_rfc3339, GlobalArgs,
    OutputMode, RunOutcome,
};
use crate::adapter::Adapter;
use crate::store::Store;
use crate::{DetailedIngestReport, IngestOptions, IngestReport};

/// Flags of the scan verb, on top of the shared ones.
#[derive(Debug, Args)]
pub struct ScanArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    /// Only pick up sessions last active at or after this cutoff: an RFC
    /// 3339 timestamp, an ISO date (YYYY-MM-DD), or a relative duration
    /// like 7d, 24h, 30m. Nothing already stored is ever deleted.
    #[arg(long, value_name = "CUTOFF")]
    pub since: Option<String>,

    /// Store path (default: $BOTSPY_HOME/.botspy/store.db).
    #[arg(long, value_name = "PATH")]
    pub store: Option<PathBuf>,

    /// Report what would change without writing.
    #[arg(long)]
    pub dry_run: bool,
}

/// Run one scan: the store's CDC pass over the selected adapters.
pub fn scan(args: ScanArgs) -> RunOutcome {
    if let Some(out) = check_root(&args.global) {
        return out;
    }
    let names = match selected_sources(&args.global.source) {
        Ok(names) => names,
        Err(err) => return RunOutcome::fail(1, format!("error: {err}\n")),
    };
    let cutoff = match &args.since {
        Some(value) => Some(match parse_cutoff(value) {
            Ok(cutoff) => cutoff,
            Err(reason) => {
                return RunOutcome::fail(2, format!("error: invalid value for --since: {reason}\n"))
            }
        }),
        None => None,
    };
    let options = args.global.options();
    let store_path = args
        .store
        .clone()
        .unwrap_or_else(|| home_of(&options).join(".botspy").join("store.db"));
    let mut adapters: Vec<Arc<dyn Adapter>> = Vec::new();
    for name in &names {
        match real_source(name, &options) {
            Ok(source) => adapters.push(Arc::from(source)),
            Err(err) => return RunOutcome::fail(1, format!("error: {err}\n")),
        }
    }
    let store = match Store::open(&store_path) {
        Ok(store) => store,
        Err(err) => return RunOutcome::fail(3, format!("error: {err}\n")),
    };
    let pass_options = IngestOptions {
        since: cutoff.clone(),
        dry_run: args.dry_run,
        prune: false,
    };
    let started = Instant::now();
    let report = match store.refresh_with(&adapters, &pass_options) {
        Ok(report) => report,
        Err(err) => return RunOutcome::fail(3, format!("error: {err}\n")),
    };
    let elapsed = started.elapsed();
    let store = store_path.display().to_string();
    match args.global.output {
        OutputMode::Json => {
            let output = scan_json(
                &names,
                &store,
                cutoff.as_deref(),
                args.dry_run,
                &report,
                elapsed,
            );
            RunOutcome::ok(format!(
                "{}\n",
                serde_json::to_string_pretty(&output).expect("scan report serializes")
            ))
        }
        OutputMode::Ndjson => {
            let output = scan_json(
                &names,
                &store,
                cutoff.as_deref(),
                args.dry_run,
                &report,
                elapsed,
            );
            let lines: Vec<String> = output
                .adapters
                .iter()
                .map(|entry| serde_json::to_string(entry).expect("adapter entry serializes"))
                .collect();
            RunOutcome::ok(format!("{}\n", lines.join("\n")))
        }
        OutputMode::Human => {
            RunOutcome::ok(scan_human(&names, &store, &report, elapsed, args.dry_run))
        }
    }
}

/// The JSON scan report: the store's own report types plus the scan's
/// own flags, store path, and elapsed time.
#[derive(serde::Serialize)]
struct ScanOutput {
    store: String,
    since: Option<String>,
    dry_run: bool,
    elapsed_ms: u64,
    totals: IngestReport,
    adapters: Vec<Value>,
}

fn scan_json(
    names: &[String],
    store: &str,
    since: Option<&str>,
    dry_run: bool,
    report: &DetailedIngestReport,
    elapsed: std::time::Duration,
) -> ScanOutput {
    let adapters: Vec<Value> = names
        .iter()
        .zip(&report.adapters)
        .map(|(name, row)| {
            let mut value = serde_json::to_value(row).expect("the adapter report serializes");
            value["source"] = Value::String(name.clone());
            value
        })
        .collect();
    ScanOutput {
        store: store.to_string(),
        since: since.map(str::to_string),
        dry_run,
        elapsed_ms: elapsed.as_millis() as u64,
        totals: report.totals,
        adapters,
    }
}

fn scan_human(
    names: &[String],
    store: &str,
    report: &DetailedIngestReport,
    elapsed: std::time::Duration,
    dry_run: bool,
) -> String {
    let mut cells: Vec<[String; 7]> = vec![[
        "SOURCE".to_string(),
        "ADDED".to_string(),
        "UPDATED".to_string(),
        "UNCHANGED".to_string(),
        "SKIPPED".to_string(),
        "ERRORS".to_string(),
        "MESSAGES".to_string(),
    ]];
    for (name, row) in names.iter().zip(&report.adapters) {
        let messages = row.added_messages as i64 + row.updated_message_delta;
        cells.push([
            name.clone(),
            row.added_sessions.to_string(),
            row.updated_sessions.to_string(),
            row.unchanged_sessions.to_string(),
            row.skipped_sessions.to_string(),
            row.errors.to_string(),
            signed(messages),
        ]);
    }
    let width = |column: usize| {
        render::column_width(
            &cells
                .iter()
                .map(|row| row[column].as_str())
                .collect::<Vec<_>>(),
        )
    };
    let (added_w, updated_w, unchanged_w, skipped_w, errors_w) =
        (width(1), width(2), width(3), width(4), width(5));
    let mut out = format!("store  {store}\n");
    for cell in &cells {
        out.push_str(&format!(
            "{}  {}  {}  {}  {}  {}  {}\n",
            render::column(&cell[0], 12),
            render::number_str(&cell[1], added_w),
            render::number_str(&cell[2], updated_w),
            render::number_str(&cell[3], unchanged_w),
            render::number_str(&cell[4], skipped_w),
            render::number_str(&cell[5], errors_w),
            cell[6]
        ));
    }
    let totals = &report.totals;
    out.push_str(&format!(
        "scanned {} in {:.1}s: {} added, {} updated, {} unchanged, {} skipped, {} errors",
        plural(names.len(), "source"),
        elapsed.as_secs_f64(),
        totals.new,
        totals.updated,
        totals.unchanged,
        totals.skipped,
        report.adapters.iter().map(|row| row.errors).sum::<usize>(),
    ));
    if dry_run {
        out.push_str(" (dry run — nothing written)");
    }
    out.push('\n');
    out
}

fn signed(value: i64) -> String {
    if value > 0 {
        format!("+{value}")
    } else if value < 0 {
        format!("{value}")
    } else {
        "0".to_string()
    }
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// The cutoff a `--since` value resolves to: the RFC 3339 timestamp
/// string the store's pass compares the library's recorded timestamps
/// against. An ISO date stands for its midnight UTC; a relative duration
/// stands for that long before now.
fn parse_cutoff(value: &str) -> Result<String, String> {
    if valid_rfc3339(value) {
        return Ok(value.to_string());
    }
    if is_iso_date(value) {
        return Ok(format!("{value}T00:00:00Z"));
    }
    if let Some(seconds) = relative_duration(value) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_secs();
        let cutoff = now.saturating_sub(seconds);
        return Ok(rfc3339_utc(cutoff as i64));
    }
    Err(format!(
        "{value:?} is not an RFC 3339 timestamp, an ISO date (YYYY-MM-DD), or a relative duration like 7d ({})",
        unit_names()
    ))
}

/// The relative-duration units a cutoff accepts, for the usage message.
fn unit_names() -> &'static str {
    "accepted units: s, m (minutes), h, d, w"
}

/// Whether a value is an ISO date: `YYYY-MM-DD` with a plausible month
/// and day.
fn is_iso_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 10 {
        return false;
    }
    let digits = |start: usize, len: usize| b[start..start + len].iter().all(u8::is_ascii_digit);
    if b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !digits(0, 4) || !digits(5, 2) || !digits(8, 2) {
        return false;
    }
    let month = (b[5] - b'0') * 10 + (b[6] - b'0');
    let day = (b[8] - b'0') * 10 + (b[9] - b'0');
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// A relative duration like `7d`: its length in seconds.
fn relative_duration(value: &str) -> Option<u64> {
    let (digits, unit) = value.split_at(value.len().checked_sub(1)?);
    let unit = unit.chars().next()?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let multiplier: u64 = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3600,
        'd' => 86400,
        'w' => 604800,
        _ => return None,
    };
    let count: u64 = digits.parse().ok()?;
    count.checked_mul(multiplier)
}

/// Format seconds since the epoch as an RFC 3339 UTC timestamp.
fn rfc3339_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let time = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z",
        h = time / 3600,
        m = (time % 3600) / 60,
        s = time % 60
    )
}

/// Days since the epoch to the (proleptic Gregorian) civil date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_pointer = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_pointer + 2) / 5 + 1) as u32;
    let month = if month_pointer < 10 {
        month_pointer + 3
    } else {
        month_pointer - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_rfc3339_cutoff_passes_through() {
        assert_eq!(
            parse_cutoff("2026-09-26T12:00:00Z").expect("valid"),
            "2026-09-26T12:00:00Z"
        );
    }

    #[test]
    fn an_iso_date_cutoff_is_its_midnight_utc() {
        assert_eq!(
            parse_cutoff("2026-09-26").expect("valid"),
            "2026-09-26T00:00:00Z"
        );
    }

    #[test]
    fn a_relative_cutoff_is_that_long_before_now() {
        let cutoff = parse_cutoff("0s").expect("valid");
        assert!(
            valid_rfc3339(&cutoff),
            "the resolved cutoff is RFC 3339: {cutoff}"
        );
        let cutoff = parse_cutoff("7d").expect("valid");
        assert!(
            valid_rfc3339(&cutoff),
            "the resolved cutoff is RFC 3339: {cutoff}"
        );
    }

    #[test]
    fn an_off_shape_cutoff_is_rejected() {
        for value in [
            "bogus",
            "",
            "7x",
            "-7d",
            "2026-13-01",
            "2026-09-26T",
            "1e3d",
        ] {
            assert!(parse_cutoff(value).is_err(), "must reject {value}");
        }
    }

    #[test]
    fn epoch_formats_as_rfc3339() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(86_400), "1970-01-02T00:00:00Z");
        assert_eq!(rfc3339_utc(-1), "1969-12-31T23:59:59Z");
        assert_eq!(rfc3339_utc(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(rfc3339_utc(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(rfc3339_utc(1_790_812_800), "2026-10-01T00:00:00Z");
    }

    #[test]
    fn the_agent_name_maps_to_the_registry_name() {
        for name in super::super::ALL_SOURCES {
            assert_eq!(
                super::super::agent_name(super::super::agent_of(name).expect("known name")),
                name
            );
        }
    }
}
