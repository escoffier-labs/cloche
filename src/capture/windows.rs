#![cfg(target_os = "windows")]

//! Windows backend design stub.
//!
//! Planned responsibilities:
//! - Discover the foreground window with Win32 APIs.
//! - Capture the target window image without including unrelated desktop
//!   background.
//! - Collect title, process id, process name, and window geometry.
//! - Extract best-effort accessibility text through UI Automation.
//! - Return the same appshots output contract used by the Linux backend.

pub struct WindowsCaptureBackend;
