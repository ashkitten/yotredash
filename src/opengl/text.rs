//! Contains a GPU cache implementation and methods for rendering strings on the screen using
//! OpenGL

use failure::{bail, Error};
use glium::{
    backend::Facade,
    implement_vertex,
    index::{NoIndices, PrimitiveType},
    program::ProgramCreationInput,
    texture::{
        MipmapsOption, PixelValue, RawImage2d, Texture2dDataSource, UncompressedFloatFormat,
    },
    uniforms::MagnifySamplerFilter,
    Blend, DrawParameters, Program, Surface, Texture2d, VertexBuffer,
};
use rect_packer::{self, DensePacker};
use std::{borrow::Cow, cmp::max, collections::HashMap, rc::Rc};

use super::UniformsStorageVec;
use crate::font::{FreeTypeRasterizer, GlyphLoader, RenderedGlyph};

const VERTEX: &str = "
    #version 140

    in vec2 position;
    in vec2 tex_coords;
    out vec2 texCoords;

    uniform mat4 projection;

    void main() {
        gl_Position = projection * vec4(position, 0.0, 1.0);
        texCoords = tex_coords;
    }
";

const FRAGMENT: &str = "
    #version 140

    in vec2 texCoords;
    out vec4 color;

    uniform sampler2D glyphTexture;
    uniform vec4 glyphColor;

    void main() {
        vec4 sampled = vec4(1.0, 1.0, 1.0, texture(glyphTexture, texCoords).r);
        color = glyphColor * sampled;
    }
";

impl<'a> Texture2dDataSource<'a> for &'a RenderedGlyph {
    type Data = u8;

    fn into_raw(self) -> RawImage2d<'a, u8> {
        RawImage2d {
            data: Cow::Borrowed(&self.buffer),
            width: self.width as u32,
            height: self.height as u32,
            format: <u8 as PixelValue>::get_format(),
        }
    }
}

/// Data about a glyph stored in the texture cache
#[derive(Clone, Debug)]
pub struct GlyphData {
    /// Rectangle containing the glyph within the cache texture
    pub rect: rect_packer::Rect,
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
    /// Line height
    pub line_height: u32,
}

/// A cache of glyphs on the GPU
pub struct GlyphCache<L: GlyphLoader> {
    /// The `Facade` it uses to access the OpenGL context
    facade: Rc<dyn Facade>,
    /// The cache in which rendered glyphs are stored
    cache: HashMap<char, GlyphData>,
    /// The texture on which the rendered glyphs are stored
    texture: Texture2d,
    /// A reference to the loader this GlyphCache uses to load new glyphs
    loader: L,
    /// The packer used to pack glyphs into the texture
    packer: DensePacker,
}

impl<L: GlyphLoader> GlyphCache<L> {
    /// Create a new instance
    pub fn new(facade: &Rc<dyn Facade>, loader: L) -> Result<Self, Error> {
        let mut cache = Self {
            facade: Rc::clone(facade),
            cache: HashMap::new(),
            loader: loader,
            packer: DensePacker::new(512, 512),
            texture: Texture2d::empty_with_format(
                &**facade,
                UncompressedFloatFormat::U8,
                MipmapsOption::NoMipmap,
                512,
                512,
            )?,
        };

        // Prerender all visible ascii characters
        for i in 32u8..127u8 {
            cache.insert(i as char)?;
        }

        Ok(cache)
    }

    /// Get a `&GlyphData` corresponding to the char code
    pub fn get(&mut self, key: char) -> Result<&GlyphData, Error> {
        if self.cache.contains_key(&key) {
            Ok(&self.cache[&key])
        } else {
            Ok(self.insert(key)?)
        }
    }

    /// Insert a new glyph into the cache texture from the loader, and return a reference to it
    pub fn insert(&mut self, key: char) -> Result<&GlyphData, Error> {
        let rendered = self.loader.load(key)?;

        if rendered.width == 0 || rendered.height == 0 {
            self.cache.insert(
                key,
                GlyphData {
                    rect: rect_packer::Rect {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    },
                    width: rendered.width,
                    height: rendered.height,
                    bearing_x: rendered.bearing_x,
                    bearing_y: rendered.bearing_y,
                    advance: rendered.advance,
                    line_height: rendered.line_height,
                },
            );
            return Ok(&self.cache[&key]);
        }

        if !self
            .packer
            .can_pack(rendered.width as i32, rendered.height as i32, false)
        {
            let old_size = (self.packer.size().0 as u32, self.packer.size().1 as u32);
            // Let new size be at least 2x the old size so we're not resizing so much
            let new_size = (
                max(old_size.0 + rendered.width, old_size.0 * 2),
                max(old_size.1 + rendered.height, old_size.1 * 2),
            );

            self.packer.resize(new_size.0 as i32, new_size.1 as i32);

            self.texture = {
                let new_texture = Texture2d::empty_with_format(
                    &*self.facade,
                    UncompressedFloatFormat::U8,
                    MipmapsOption::NoMipmap,
                    new_size.0,
                    new_size.1,
                )?;
                let blit_rect = ::glium::Rect {
                    left: 0,
                    bottom: 0,
                    width: old_size.0,
                    height: old_size.1,
                };
                let blit_target = ::glium::BlitTarget {
                    left: 0,
                    bottom: 0,
                    width: old_size.0 as i32,
                    height: old_size.1 as i32,
                };
                new_texture.as_surface().blit_from_simple_framebuffer(
                    &self.texture.as_surface(),
                    &blit_rect,
                    &blit_target,
                    MagnifySamplerFilter::Nearest,
                );
                new_texture
            };
        }

        if let Some(rect) = self
            .packer
            .pack(rendered.width as i32, rendered.height as i32, false)
        {
            let blit_source = Texture2d::with_format(
                &*self.facade,
                &rendered,
                UncompressedFloatFormat::U8,
                MipmapsOption::NoMipmap,
            )?;
            let blit_rect = ::glium::Rect {
                left: 0,
                bottom: 0,
                width: rendered.width as u32,
                height: rendered.height as u32,
            };
            let blit_target = ::glium::BlitTarget {
                left: rect.x as u32,
                bottom: rect.y as u32,
                width: rect.width,
                height: rect.height,
            };
            self.texture.as_surface().blit_from_simple_framebuffer(
                &blit_source.as_surface(),
                &blit_rect,
                &blit_target,
                MagnifySamplerFilter::Nearest,
            );

            self.cache.insert(
                key,
                GlyphData {
                    rect: rect,
                    width: rendered.width,
                    height: rendered.height,
                    bearing_x: rendered.bearing_x,
                    bearing_y: rendered.bearing_y,
                    advance: rendered.advance,
                    line_height: rendered.line_height,
                },
            );
            Ok(&self.cache[&key])
        } else {
            bail!("Failed to pack texture");
        }
    }
}

/// An implementation of vertex attributes needed for rendering text
#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);

/// The actual `TextRenderer` which uses a `Program` and a `GlyphCache` to render glyphs on a
/// given surface
pub struct TextRenderer {
    /// The `Facade` it uses to access the OpenGL context
    facade: Rc<dyn Facade>,
    /// The `GlyphCache` which it uses to store rendered glyphs
    glyph_cache: GlyphCache<FreeTypeRasterizer>,
    /// The shader program it uses for drawing
    program: Program,
}

impl TextRenderer {
    /// Create a new instance using a specified font and size
    pub fn new(facade: &Rc<dyn Facade>, font: &str, font_size: f32) -> Result<Self, Error> {
        let glyph_cache = GlyphCache::new(
            &Rc::clone(&facade),
            FreeTypeRasterizer::new(font, font_size)?,
        )?;

        let program = {
            let input = ProgramCreationInput::SourceCode {
                vertex_shader: VERTEX,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                geometry_shader: None,
                fragment_shader: FRAGMENT,
                transform_feedback_varyings: None,
                outputs_srgb: true,
                uses_point_size: false,
            };
            Program::new(&**facade, input)?
        };

        Ok(Self {
            facade: Rc::clone(facade),
            glyph_cache,
            program,
        })
    }

    /// Draw text on the surface at specified XY coordinates and with a specified color
    pub fn draw_text<S>(
        &mut self,
        surface: &mut S,
        text: &str,
        pos: [f32; 2],
        color: [f32; 4],
    ) -> Result<(), Error>
    where
        S: Surface,
    {
        let (x, y) = (pos[0], pos[1]);
        let mut advance_x = 0;
        let mut advance_y = 0;
        for c in text.chars() {
            let glyph = self.glyph_cache.get(c)?.clone();

            // Special case for carriage return
            if c == '\n' {
                advance_y += glyph.line_height;
                advance_x = 0;
                continue;
            }

            if glyph.width != 0 && glyph.height != 0 {
                let (win_width, win_height) = surface.get_dimensions();
                let p_x = 2.0 / win_width as f32;
                let p_y = 2.0 / win_height as f32;

                // Rows translate to columns in glsl
                #[cfg_attr(rustfmt, rustfmt_skip)]
                let projection = [
                    [ p_x,  0.0,  0.0,  0.0],
                    [ 0.0,  p_y,  0.0,  0.0],
                    [ 0.0,  0.0,  1.0,  0.0],
                    [-1.0, -1.0,  0.0,  1.0],
                ];

                let mut uniforms = UniformsStorageVec::new();
                uniforms.push("glyphColor", color);
                uniforms.push("glyphTexture", self.glyph_cache.texture.sampled());
                uniforms.push("projection", projection);

                let x = x + (glyph.bearing_x + advance_x) as f32;
                let y = y + glyph.bearing_y as f32 - advance_y as f32 - glyph.line_height as f32
                    + win_height as f32;
                let w = glyph.width as f32;
                let h = glyph.height as f32;

                let t_x1 = glyph.rect.x as f32 / self.glyph_cache.texture.width() as f32;
                let t_x2 = (glyph.rect.x as f32 + glyph.rect.width as f32)
                    / self.glyph_cache.texture.width() as f32;
                let t_y1 = glyph.rect.y as f32 / self.glyph_cache.texture.height() as f32;
                let t_y2 = (glyph.rect.y as f32 + glyph.rect.height as f32)
                    / self.glyph_cache.texture.height() as f32;

                #[cfg_attr(rustfmt, rustfmt_skip)]
                let vertices = [
                    Vertex { position: [x    , y + h], tex_coords: [t_x1, t_y1] },
                    Vertex { position: [x    , y    ], tex_coords: [t_x1, t_y2] },
                    Vertex { position: [x + w, y    ], tex_coords: [t_x2, t_y2] },
                    Vertex { position: [x    , y + h], tex_coords: [t_x1, t_y1] },
                    Vertex { position: [x + w, y    ], tex_coords: [t_x2, t_y2] },
                    Vertex { position: [x + w, y + h], tex_coords: [t_x2, t_y1] },
                ];

                let vertex_buffer = VertexBuffer::new(&*self.facade, &vertices)?;
                let index_buffer = NoIndices(PrimitiveType::TrianglesList);

                let params = DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                };

                surface.draw(
                    &vertex_buffer,
                    &index_buffer,
                    &self.program,
                    &uniforms,
                    &params,
                )?;
            }

            advance_x += glyph.advance as i32;
        }

        Ok(())
    }
}
