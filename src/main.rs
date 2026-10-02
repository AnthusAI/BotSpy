//! Thin binary over the `botspy` library: parse arguments, run one
//! verb, print the outcome, exit with its code. All behavior lives in
//! the library (`botspy::cli`), where the specifications exercise it.

fn main() -> std::process::ExitCode {
    let outcome = botspy::cli::run_from(std::env::args_os());
    if !outcome.stdout.is_empty() {
        print!("{}", outcome.stdout);
    }
    if !outcome.stderr.is_empty() {
        eprint!("{}", outcome.stderr);
    }
    std::process::ExitCode::from(outcome.code.clamp(0, 255) as u8)
}