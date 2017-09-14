use freetype::Library;
use freetype::face::Face;
use std::collections::HashMap;
use std::rc::Rc;

use errors::*;

pub struct RenderedGlyph {
    /// Bitmap buffer (format: U8)
    pub buffer: Vec<u8>,
    /// Width of bitmap in pixels
    pub width: u32,
    /// Number of rows in bitmap
    pub rows: u32,
    /// Additional distance from left
    pub bearing_x: f32,
    /// Additional distance from top
    pub bearing_y: f32,
    /// Advance distance to start of next character
    pub advance: f32,
}

pub trait GlyphLoader {
    /// Creates a new instance of the GlyphCache
    fn new(path: &str, size: u32) -> Result<Self>
    where
        Self: Sized;
    /// Loads a glyph and renders it
    fn load(&self, key: usize) -> Result<RenderedGlyph>;
}

pub struct GlyphCache {
    /// The cache in which rendered glyphs are stored
    cache: HashMap<usize, RenderedGlyph>,
    /// A reference to the loader this GlyphCache uses to load new glyphs
    loader: Rc<GlyphLoader>,
}

impl GlyphCache {
    pub fn new<L: GlyphLoader + 'static>(loader: Rc<L>) -> Result<Self> {
        let mut cache = Self {
            cache: HashMap::new(),
            loader: loader,
        };

        // Prerender all visible ascii characters
        // TODO: change to `32..=127` when inclusive ranges make it to stable Rust
        for i in 32..128usize {
            cache.get(i)?;
        }

        Ok(cache)
    }

    pub fn get(&mut self, key: usize) -> Result<&RenderedGlyph> {
        Ok(self.cache.entry(key).or_insert(self.loader.load(key)?))
    }
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
            rows: slot.bitmap().rows() as u32,
            bearing_x: slot.bitmap_left() as f32,
            bearing_y: slot.bitmap_top() as f32,
            // TODO: figure out why I need to divide by 2.0
            advance: slot.advance().x as f32 / 26.6 / 2.0,
        })
    }
}
