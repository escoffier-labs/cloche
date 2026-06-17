//! `cloche setup`: install the hotkey, register the MCP server, and verify.

use std::process::ExitCode;

use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;

pub mod agents;
pub mod flow;
pub mod hotkey;
pub mod verify;

#[derive(Debug, Args)]
pub struct SetupArgs {
    #[command(subcommand)]
    pub command: Option<SetupCommand>,
    /// Apply changes without the confirmation prompt.
    #[arg(long, global = true)]
    pub yes: bool,
    /// Print every change that would be made and exit without changing anything.
    #[arg(long, global = true)]
    pub print: bool,
    /// Output format. `text` is the human default; `json` emits the SetupReport.
    #[arg(long, global = true, value_enum, default_value = "text")]
    pub format: SetupFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SetupFormat {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
pub enum SetupCommand {
    /// Install and bind the screenshot hotkey only.
    Hotkey,
    /// Register the MCP server with agent clients only.
    Agent(AgentArgs),
    /// Re-run the confirmation checks only.
    Verify,
}

#[derive(Debug, Args)]
pub struct AgentArgs {
    /// Configure a specific client; auto-detects all installed clients when omitted.
    #[arg(long, value_enum)]
    pub client: Option<AgentClient>,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AgentClient {
    ClaudeCode,
    Openclaw,
    Codex,
    Print,
}

pub fn run(args: SetupArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    flow::run(args)
}
