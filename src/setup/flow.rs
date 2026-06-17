//! The guided `cloche setup` flow: detect -> plan -> confirm -> apply -> verify.

use std::io::Write;
use std::process::ExitCode;

use serde::Serialize;
use serde_json::json;

use crate::setup::SetupArgs;
use crate::setup::SetupCommand;
use crate::setup::SetupFormat;
use crate::setup::agents;
use crate::setup::agents::ClientStatus;
use crate::setup::hotkey;
use crate::setup::verify;
use crate::setup::verify::CheckStatus;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupReport {
    ok: bool,
    mode: String,
    applied: Vec<String>,
    skipped: Vec<String>,
    printed: Vec<String>,
    backups: Vec<String>,
    checks: Vec<serde_json::Value>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

pub fn run(args: SetupArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    // `verify` and `--print` never mutate, so they never prompt.
    let is_verify_only = matches!(args.command, Some(SetupCommand::Verify));
    let apply = !args.print && !is_verify_only;
    if apply && !args.yes && !confirm_prompt()? {
        // Emit a valid report so `--format json` stays parseable on decline.
        let report = SetupReport {
            ok: true,
            mode: mode_label(&args.command),
            applied: vec![],
            skipped: vec![],
            printed: vec![],
            backups: vec![],
            checks: vec![],
            warnings: vec!["aborted by user; nothing changed".into()],
            errors: vec![],
        };
        eprintln!("Aborted. Nothing changed.");
        emit(&report, args.format)?;
        return Ok(ExitCode::SUCCESS);
    }
    let mut report = SetupReport {
        ok: true,
        mode: mode_label(&args.command),
        applied: vec![],
        skipped: vec![],
        printed: vec![],
        backups: vec![],
        checks: vec![],
        warnings: vec![],
        errors: vec![],
    };

    let do_hotkey = matches!(args.command, None | Some(SetupCommand::Hotkey));
    let do_agents = matches!(args.command, None | Some(SetupCommand::Agent(_)));
    let do_verify = matches!(args.command, None | Some(SetupCommand::Verify));

    if do_hotkey {
        let (outcome, warns) = hotkey::setup_hotkey(apply);
        report.warnings.extend(warns);
        match outcome {
            hotkey::HotkeyOutcome::Bound { changed: true } => {
                report.applied.push("hotkey:gnome-binding".into())
            }
            hotkey::HotkeyOutcome::Bound { changed: false } => {
                report.skipped.push("hotkey:already-bound".into())
            }
            hotkey::HotkeyOutcome::Manual => {
                report.printed.push("hotkey:manual-instructions".into())
            }
        }
    }

    if do_agents {
        let only = match &args.command {
            Some(SetupCommand::Agent(a)) => a.client,
            _ => None,
        };
        for r in agents::setup_agents(only, apply) {
            match r.status {
                ClientStatus::Applied => report.applied.push(format!("agent:{}", r.client)),
                ClientStatus::AlreadyConfigured => {
                    report.skipped.push(format!("agent:{}", r.client))
                }
                ClientStatus::Printed => report.printed.push(format!("agent:{}", r.client)),
                ClientStatus::Error => {
                    report
                        .errors
                        .push(format!("agent:{}: {}", r.client, r.message));
                }
            }
            if let Some(b) = r.backup {
                report.backups.push(b.display().to_string());
            }
        }
    }

    if do_verify && !args.print {
        for c in verify::run_all() {
            if c.status == CheckStatus::Fail {
                report.ok = false;
            }
            report.checks.push(json!({
                "name": c.name,
                "status": match c.status {
                    CheckStatus::Pass => "pass",
                    CheckStatus::Fail => "fail",
                    CheckStatus::Skip => "skip",
                },
                "detail": c.detail,
            }));
        }
    }

    if !report.errors.is_empty() {
        report.ok = false;
    }
    emit(&report, args.format)?;
    Ok(if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn mode_label(cmd: &Option<SetupCommand>) -> String {
    match cmd {
        None => "setup",
        Some(SetupCommand::Hotkey) => "hotkey",
        Some(SetupCommand::Agent(_)) => "agent",
        Some(SetupCommand::Verify) => "verify",
    }
    .to_string()
}

fn confirm_prompt() -> Result<bool, std::io::Error> {
    // Prompt on stderr so `--format json` stdout stays pure JSON.
    let desktop = hotkey::detect_desktop();
    eprintln!("Cloche setup will, on this {desktop:?} session:");
    eprintln!("  - install ~/.local/bin/cloche-grab and bind it to Print (GNOME) or print steps");
    eprintln!("  - register the cloche MCP server with detected agents (backs up edited files)");
    eprintln!("  - verify capture, hotkey, and MCP");
    eprint!("Proceed? [y/N] ");
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn emit(report: &SetupReport, format: SetupFormat) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        SetupFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
        SetupFormat::Text => {
            for a in &report.applied {
                println!("applied: {a}");
            }
            for s in &report.skipped {
                println!("ok (already done): {s}");
            }
            for p in &report.printed {
                println!("manual step printed: {p}");
            }
            for c in &report.checks {
                println!(
                    "check {}: {} - {}",
                    c["name"].as_str().unwrap_or(""),
                    c["status"].as_str().unwrap_or(""),
                    c["detail"].as_str().unwrap_or("")
                );
            }
            for w in &report.warnings {
                println!("warning: {w}");
            }
            for e in &report.errors {
                println!("error: {e}");
            }
            println!(
                "{}",
                if report.ok {
                    "Setup OK."
                } else {
                    "Setup finished with problems (see above)."
                }
            );
        }
    }
    Ok(())
}
