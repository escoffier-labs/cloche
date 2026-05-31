mod backends;
mod capture;
mod cli;
mod codex;
mod contract;
mod polish;
mod text;
mod util;

use std::process::ExitCode;

use clap::Parser;
use cli::Cli;
use cli::Command;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("appshots: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Capture(args) => cli::capture(args),
        Command::Doctor(args) => cli::doctor(args),
        Command::ListWindows(args) => cli::list_windows(args),
        Command::Gallery(args) => cli::gallery(args),
        Command::Latest(args) => cli::latest(args),
        Command::Preview(args) => cli::preview(args),
        Command::Schema(args) => cli::schema(args),
        Command::CodexPayload(args) => codex::payload(args),
    }
}
