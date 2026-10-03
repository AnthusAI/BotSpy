//! Thin binary over the `botspy` library: parse arguments, run one
//! verb, print the outcome, exit with its code. All behavior lives in
//! the library (`botspy::cli`), where the specifications exercise it.
//!
//! A scan loop streams each pass's report as it finishes; every other
//! outcome is printed once at the end.

use std::io::Write;

fn main() -> std::process::ExitCode {
    let mut scan_sink = |block: &str| {
        print!("{block}");
        let _ = std::io::stdout().flush();
    };
    let outcome = botspy::cli::run_from_with_sink(std::env::args_os(), &mut scan_sink);
    if !outcome.stdout.is_empty() {
        print!("{}", outcome.stdout);
    }
    if !outcome.stderr.is_empty() {
        eprint!("{}", outcome.stderr);
    }
    std::process::ExitCode::from(outcome.code.clamp(0, 255) as u8)
}
