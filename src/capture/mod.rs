//! Platform capture backends.
//!
//! The current implementation lives in `backends.rs` while the Linux MVP
//! settles. New platform implementations should move behind this module and
//! keep the public CLI contract stable across operating systems.

#[cfg(target_os = "windows")]
pub mod windows;
