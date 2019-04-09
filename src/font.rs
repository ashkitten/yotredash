//! Provides methods and structs for loading fonts.

use euclid::{Point2D, Size2D};
use failure::Error;
use font_kit::{
    canvas::{Canvas, Format, RasterizationOptions},
    family_name::FamilyName,
    font::Font,
    hinting::HintingOptions,
    properties::Properties,
    source::SystemSource,
};

/// Contains information about a rendered glyph, including a buffer of pixel data to load into a
/// texture
#[derive(Clone, Debug)]
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
    pub advance: u32,
    /// Line height of font
    pub line_height: u32,
}

/// Generic loader for glyphs
pub trait GlyphLoader {
    /// Creates a new instance of the GlyphCache
    fn new(path: &str, size: f32) -> Result<Self, Error>
    where
        Self: Sized;
    /// Loads a glyph and renders it
    fn load(&self, character: char) -> Result<RenderedGlyph, Error>;
}

/// A `GlyphLoader` implementation that uses the `FreeType` library to load and render glyphs
pub struct FreeTypeRasterizer {
    font: Font,
    size: f32,
}

impl GlyphLoader for FreeTypeRasterizer {
    fn new(font_name: &str, size: f32) -> Result<Self, Error> {
        let font = SystemSource::new()
            .select_best_match(
                &[
                    FamilyName::Title(font_name.to_string()),
                    FamilyName::Monospace,
                ],
                &Properties::new(),
            )
            .unwrap()
            .load()?;

        Ok(Self { font, size })
    }

    fn load(&self, key: char) -> Result<RenderedGlyph, Error> {
        let glyph_id = self.font.glyph_for_char(key).unwrap();

        let raster_bounds = self.font.raster_bounds(
            glyph_id,
            self.size,
            &Point2D::zero(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )?;

        let mut canvas = Canvas::new(
            &Size2D::new(
                raster_bounds.size.width as u32,
                raster_bounds.size.height as u32,
            ),
            Format::A8,
        );

        self.font.rasterize_glyph(
            &mut canvas,
            glyph_id,
            self.size,
            &Point2D::zero(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )?;

        let metrics = self.font.metrics();
        let scale = metrics.units_per_em as f32 / self.size;

        Ok(RenderedGlyph {
            buffer: canvas.pixels,
            width: canvas.size.width as u32,
            height: canvas.size.height as u32,
            bearing_x: raster_bounds.origin.x as i32,
            bearing_y: raster_bounds.origin.y as i32,
            advance: (self.font.advance(glyph_id)?.x / scale) as u32,
            line_height: ((self.size / (metrics.ascent + metrics.descent)) * metrics.ascent) as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::font::{FreeTypeRasterizer, GlyphLoader};

    #[test]
    fn renders_glyphs() {
        let rasterizer = FreeTypeRasterizer::new("", 20.0).unwrap();

        for c in ['F', 'U', 'C', 'K'].iter() {
            let glyph = rasterizer.load(*c).unwrap();
            let (w, h) = (glyph.width as usize, glyph.height as usize);
            println!("{:?}", glyph);
            for y in 0..h {
                for x in 0..w {
                    let _val = glyph.buffer[(y * w) + x];
                    #[rustfmt::skip]
                    print!(
                        "{0}{0}",
                        match glyph.buffer[(y * w) + x] {
                              0..=51  => ' ',
                             52..=102 => '░',
                            103..=153 => '▒',
                            154..=204 => '▓',
                            205..=255 => '█',
                        }
                    );
                }
                println!();
            }
        }
    }
}
