use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use serde_json::json;

use crate::contract::AppshotResult;
use crate::contract::ImageDetail;
use crate::util;

#[derive(Debug, Args)]
pub struct CodexPayloadArgs {
    #[arg(long)]
    pub thread_id: String,
    #[arg(value_name = "CAPTURE_DIR")]
    pub capture_dir: PathBuf,
    #[arg(long, default_value = "Appshot attached.")]
    pub message: String,
    #[arg(long, value_enum, default_value = "high")]
    pub detail: ImageDetail,
    #[arg(long)]
    pub compact: bool,
}

pub fn payload(args: CodexPayloadArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let metadata_path = args.capture_dir.join("metadata.json");
    let metadata_bytes = util::read(&metadata_path)?;
    let metadata: AppshotResult = serde_json::from_slice(&metadata_bytes)?;
    if !metadata.ok {
        return Err(
            "capture metadata is not successful; refusing to emit a Codex image payload".into(),
        );
    }
    let image_path = metadata
        .image
        .as_ref()
        .map(|image| image.path.clone())
        .ok_or("capture metadata does not include an image")?;
    let image_path = util::canonical_or_original(&image_path);
    if !image_path.is_file() {
        return Err(format!("captured image does not exist: {}", image_path.display()).into());
    }

    let mut input = vec![json!({
        "type": "text",
        "text": args.message,
        "textElements": []
    })];

    if let Some(text_path) = metadata.text.path.as_ref()
        && let Ok(text_bytes) = util::read(text_path)
    {
        let text = String::from_utf8_lossy(&text_bytes).trim().to_string();
        if !text.is_empty() {
            input.push(json!({
                "type": "text",
                "text": format!("Available app text:\n{text}"),
                "textElements": []
            }));
        }
    }

    input.push(json!({
        "type": "localImage",
        "path": image_path,
        "detail": args.detail.to_string()
    }));

    let payload = json!({
        "method": "turn/start",
        "params": {
            "threadId": args.thread_id,
            "input": input
        }
    });

    if args.compact {
        println!("{}", serde_json::to_string(&payload)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&payload)?);
    }
    Ok(ExitCode::SUCCESS)
}
