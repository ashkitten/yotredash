use failure::Error;

/// Renders a configured shader
pub trait Renderer {
    /// Do stuff like handle event queue, reload, etc
    fn update(&mut self) -> Result<(), Error>;
    /// Render the current frame
    fn render(&mut self) -> Result<(), Error>;
    /// Tells the renderer to swap buffers (only applicable to buffered renderers)
    fn swap_buffers(&self) -> Result<(), Error>;
}

/// Renders errors
pub trait DebugRenderer {
    /// Draw an error on the window
    fn draw_error(&mut self, error: &Error) -> Result<(), Error>;
}
