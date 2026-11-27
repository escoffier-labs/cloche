mod backends;
mod capture;
mod captures;
mod cli;
mod clipboard;
mod codex;
mod commands;
mod config;
mod contract;
mod html;
mod mcp;
mod pattern;
mod polish;
mod setup;
mod sky;
mod space;
mod studio;
mod terrain;
mod text;
mod util;

use std::process::ExitCode;

use clap::Parser;
use cli::Cli;

pub fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    cli::dispatch(Cli::parse())
}
