use freetype::Library;
use freetype::face::Face;

use errors::*;

#[derive(Clone)]
pub struct RenderedGlyph {
    /// Bitmap buffer (format: U8)
    pub buffer: Vec<u8>,
    /// Width of glyph in pixels
    pub width: u32,
    /// Height of glyph in pixels
    pub height: u32,
    /// Additional distance from left
    pub bearing_x: f32,
    /// Additional distance from top
    pub bearing_y: f32,
    /// Advance distance to start of next character
    pub advance: f32,
}

/// Generic loader for glyphs
pub trait GlyphLoader {
    /// Creates a new instance of the GlyphCache
    fn new(path: &str, size: u32) -> Result<Self>
    where
        Self: Sized;
    /// Loads a glyph and renders it
    fn load(&self, key: usize) -> Result<RenderedGlyph>;
}

/// A GlyphLoader implementation that uses the FreeType library to load and render glyphs
pub struct FreeTypeRasterizer {
    face: Face<'static>,
}

impl GlyphLoader for FreeTypeRasterizer {
    fn new(path: &str, size: u32) -> Result<Self> {
        let library = Library::init()?;
        let face = library.new_face(path, 0)?;

        face.set_pixel_sizes(0, size)?;

        Ok(Self { face: face })
    }

    fn load(&self, key: usize) -> Result<RenderedGlyph> {
        self.face.load_char(key, ::freetype::face::RENDER)?;
        let slot = self.face.glyph();

        Ok(RenderedGlyph {
            buffer: slot.bitmap().buffer().into(),
            width: slot.bitmap().width() as u32,
            height: slot.bitmap().rows() as u32,
            bearing_x: slot.bitmap_left() as f32,
            bearing_y: slot.bitmap_top() as f32,
            // TODO: figure out why I need to divide by 2.0
            advance: slot.advance().x as f32 / 26.6 / 2.0,
        })
    }
}
