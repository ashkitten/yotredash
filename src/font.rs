//! Provides methods and structs for loading fonts.

use freetype::Library;
use freetype::face::Face;
use owning_ref::OwningHandle;
use std::rc::Rc;
use failure::Error;

use util::DerefInner;

/// Convert from pixels to 26.6 fractional points
#[inline]
pub fn to_freetype_26_6(f: f32) -> isize {
    ((1i32 << 6) as f32 * f) as isize
}

/// Convert from 26.6 fractional points to pixels
#[inline]
pub fn from_freetype_26_6(i: isize) -> f32 {
    (i >> 6) as f32
}

/// Contains information about a rendered glyph, including a buffer of pixel data to load into a
/// texture
#[derive(Clone)]
pub struct RenderedGlyph {
    /// Bitmap buffer (format: U8)
    pub buffer: Vec<u8>,
    /// Width of glyph in pixels
    pub width: u32,
    /// Height of glyph in pixels
    pub height: u32,
    /// Additional distance from left
    pub bearing_x: i32,
    /// Additional distance from top
    pub bearing_y: i32,
    /// Advance distance to start of next character
    pub advance: f32,
}

/// Generic loader for glyphs
pub trait GlyphLoader {
    /// Creates a new instance of the GlyphCache
    fn new(path: &str, size: f32) -> Result<Self, Error>
    where
        Self: Sized;
    /// Loads a glyph and renders it
    fn load(&self, key: usize) -> Result<RenderedGlyph, Error>;
}

/// A `GlyphLoader` implementation that uses the `FreeType` library to load and render glyphs
pub struct FreeTypeRasterizer {
    face: OwningHandle<Rc<Vec<u8>>, DerefInner<Face<'static>>>,
}

impl GlyphLoader for FreeTypeRasterizer {
    fn new(font: &str, size: f32) -> Result<Self, Error> {
        let library = Library::init()?;

        let property = ::font_loader::system_fonts::FontPropertyBuilder::new()
            .family(font)
            .build();
        if let Some((font_buf, _)) = ::font_loader::system_fonts::get(&property) {
            let font_buf = Rc::new(font_buf);
            let face = OwningHandle::try_new(font_buf, |fb| unsafe {
                library.new_memory_face(&*fb, 0).map(DerefInner)
            })?;

            if let (Some(name), Some(style)) = (face.family_name(), face.style_name()) {
                debug!("Using font: {}, style: {}", name, style);
            }

            (*face).set_char_size(to_freetype_26_6(size), 0, 0, 0)?;

            Ok(Self { face: face })
        } else {
            bail!("Failed to load font");
        }
    }

    fn load(&self, key: usize) -> Result<RenderedGlyph, Error> {
        self.face
            .load_char(key, ::freetype::face::LoadFlag::RENDER)?;
        let slot = self.face.glyph();

        Ok(RenderedGlyph {
            buffer: slot.bitmap().buffer().into(),
            width: slot.bitmap().width() as u32,
            height: slot.bitmap().rows() as u32,
            bearing_x: slot.bitmap_left(),
            bearing_y: slot.bitmap_top(),
            advance: from_freetype_26_6(slot.advance().x as isize),
        })
    }
}
