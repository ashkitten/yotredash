use glium::{Blend, DrawParameters, Program, Surface, Texture2d, VertexBuffer};
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{MipmapsOption, PixelValue, RawImage2d, Texture2dDataSource, UncompressedFloatFormat};
use glium::uniforms::MagnifySamplerFilter;
use rect_packer::DensePacker;
use std::borrow::Cow;
use std::cmp::max;
use std::collections::HashMap;
use std::rc::Rc;

use super::UniformsStorageVec;
use errors::*;
use font::{FreeTypeRasterizer, GlyphLoader, RenderedGlyph};

#[derive(Clone)]
pub struct GlyphData {
    /// Rectangle containing the glyph within the cache texture
    pub rect: ::rect_packer::Rect,
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

/// A cache of glyphs on the GPU
pub struct GlyphCache {
    /// The cache in which rendered glyphs are stored
    cache: HashMap<usize, GlyphData>,
    /// The texture on which the rendered glyphs are stored
    texture: Texture2d,
    /// A reference to the loader this GlyphCache uses to load new glyphs
    loader: Rc<GlyphLoader>,
    /// The packer used to pack glyphs into the texture
    packer: DensePacker,
}

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

impl GlyphCache {
    pub fn new<L>(facade: &Facade, loader: Rc<L>) -> Result<Self>
    where
        L: GlyphLoader + 'static,
    {
        let mut cache = Self {
            cache: HashMap::new(),
            loader: loader,
            packer: DensePacker::new(512, 512),
            texture: Texture2d::empty_with_format(
                facade,
                UncompressedFloatFormat::U8,
                MipmapsOption::NoMipmap,
                512,
                512,
            )?,
        };

        // Prerender all visible ascii characters
        // TODO: change to `32..=127` when inclusive ranges make it to stable Rust
        for i in 32..128usize {
            cache.insert(i, facade)?;
        }

        Ok(cache)
    }

    pub fn get(&mut self, key: usize, facade: &Facade) -> Result<&GlyphData> {
        if self.cache.contains_key(&key) {
            Ok(self.cache.get(&key).unwrap())
        } else {
            Ok(self.insert(key, facade)?)
        }
    }

    pub fn insert(&mut self, key: usize, facade: &Facade) -> Result<&GlyphData> {
        let rendered = self.loader.load(key)?;

        if rendered.width == 0 || rendered.height == 0 {
            self.cache.insert(
                key,
                GlyphData {
                    rect: ::rect_packer::Rect {
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
                },
            );
            return Ok(self.cache.get(&key).unwrap());
        }

        if !self.packer
            .can_pack(rendered.width as i32, rendered.height as i32, false)
        {
            let old_size = (self.packer.size().0 as u32, self.packer.size().1 as u32);
            // Let new size be at least 2x the old size so we're not resizing so much
            let new_size =
                (max(old_size.0 + rendered.width, old_size.0 * 2), max(old_size.1 + rendered.height, old_size.1 * 2));

            self.packer.resize(new_size.0 as i32, new_size.1 as i32);

            self.texture = {
                let new_texture = Texture2d::empty_with_format(
                    facade,
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

        // TODO: can fail if texture is not big enough
        if let Some(rect) = self.packer
            .pack(rendered.width as i32, rendered.height as i32, false)
        {
            let blit_source =
                Texture2d::with_format(facade, &rendered, UncompressedFloatFormat::U8, MipmapsOption::NoMipmap)?;
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
                },
            );
            Ok(self.cache.get(&key).unwrap())
        } else {
            bail!("Failed to pack texture");
        }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);

pub struct TextRenderer {
    glyph_cache: GlyphCache,
    program: Program,
}

impl TextRenderer // where
{
    pub fn new<B>(facade: &B, font_path: &str, font_size: f32) -> Result<Self>
    where
        B: Facade,
    {
        let glyph_cache = GlyphCache::new(facade, Rc::new(FreeTypeRasterizer::new(font_path, font_size)?))?;

        let program = program!(facade,
            140 => {
                vertex: "
                    #version 140

                    in vec2 position;
                    in vec2 tex_coords;
                    out vec2 texCoords;

                    uniform mat4 projection;

                    void main() {
                        gl_Position = projection * vec4(position, 0.0, 1.0);
                        texCoords = tex_coords;
                    }
                ",
                fragment: "
                    #version 140

                    in vec2 texCoords;
                    out vec4 color;

                    uniform sampler2D glyphTexture;
                    uniform vec3 glyphColor;

                    void main() {
                        vec4 sampled = vec4(1.0, 1.0, 1.0, texture(glyphTexture, texCoords).r);
                        color = vec4(glyphColor, 1.0) * sampled;
                    }
                ",
            }
        )?;

        Ok(Self {
            glyph_cache: glyph_cache,
            program: program,
        })
    }

    pub fn draw_text<S>(
        &mut self, facade: &Facade, surface: &mut S, text: &str, x: f32, y: f32, color: [f32; 3]
    ) -> Result<()>
    where
        S: Surface,
    {
        let mut advance = 0;
        for c in text.chars() {
            let glyph = self.glyph_cache.get(c as usize, facade)?.clone();

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

                let x = x + (glyph.bearing_x + advance) as f32;
                let y = y - (glyph.height as f32 - glyph.bearing_y as f32);
                let w = glyph.width as f32;
                let h = glyph.height as f32;

                let t_x1 = glyph.rect.x as f32 / self.glyph_cache.texture.width() as f32;
                let t_x2 = (glyph.rect.x as f32 + glyph.rect.width as f32) / self.glyph_cache.texture.width() as f32;
                let t_y1 = glyph.rect.y as f32 / self.glyph_cache.texture.height() as f32;
                let t_y2 = (glyph.rect.y as f32 + glyph.rect.height as f32) / self.glyph_cache.texture.height() as f32;

                #[cfg_attr(rustfmt, rustfmt_skip)]
                let vertices = [
                    Vertex { position: [x    , y + h], tex_coords: [t_x1, t_y1] },
                    Vertex { position: [x    , y    ], tex_coords: [t_x1, t_y2] },
                    Vertex { position: [x + w, y    ], tex_coords: [t_x2, t_y2] },
                    Vertex { position: [x    , y + h], tex_coords: [t_x1, t_y1] },
                    Vertex { position: [x + w, y    ], tex_coords: [t_x2, t_y2] },
                    Vertex { position: [x + w, y + h], tex_coords: [t_x2, t_y1] },
                ];

                let vertex_buffer = VertexBuffer::new(facade, &vertices)?;
                let index_buffer = NoIndices(PrimitiveType::TrianglesList);

                let params = DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                };

                surface.draw(&vertex_buffer, &index_buffer, &self.program, &uniforms, &params)?;
            }

            advance += glyph.advance as i32;
        }

        Ok(())
    }
}
