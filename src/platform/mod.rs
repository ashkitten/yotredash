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

#[cfg(all(not(windows), not(unix), not(macos)))]
use this_platform_is_not_supported;
