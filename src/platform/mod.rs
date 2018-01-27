//! Contains platform-specific functionality

#![cfg_attr(feature = "cargo-clippy", allow(module_inception))]

pub use self::platform::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod platform;
#[cfg(unix)]
#[path = "unix/mod.rs"]
mod platform;
#[cfg(macos)]
#[path = "macos/mod.rs"]
mod platform;
