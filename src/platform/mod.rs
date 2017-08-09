pub use self::platform::*;

#[cfg(target_os = "windows")]
#[path="windows/mod.rs"]
mod platform;
#[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]
#[path="unix/mod.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path="macos/mod.rs"]
mod platform;

#[cfg(all(not(target_os = "windows"),
    not(target_os = "macos"),
    not(target_os = "linux"), not(target_os = "dragonfly"), not(target_os = "freebsd"), not(target_os = "openbsd")))]
use this_platform_is_not_supported;
