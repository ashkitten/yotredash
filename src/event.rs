//! Events are things that happen :D

use std::path::PathBuf;

use config::Config;

/// Events related to the mouse pointer
#[derive(Clone)]
pub enum PointerEvent {
    /// Pointer was moved to (x, y)
    Move(f32, f32),
    /// Mouse was clicked
    Press,
    /// Mouse was released
    Release,
}

/// Events related to the renderer
#[derive(Clone)]
pub enum RendererEvent {
    /// Pointer event
    Pointer(PointerEvent),
    /// Window was resized
    Resize(u32, u32),
    /// Renderer should reload from a new configuration
    Reload(Config),
    /// Renderer should capture an image to this file
    Capture(PathBuf),
}

/// All events
pub enum Event {
    /// Pointer event
    Pointer(PointerEvent),
    /// Window was resized
    Resize(u32, u32),
    /// Renderer should reload
    Reload,
    /// Renderer should capture an image
    Capture,
    /// Close the window
    Close,
}
