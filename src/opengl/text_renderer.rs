use glium::{Blend, DrawParameters, Program, Surface, Texture2d, VertexBuffer};
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{RawImage2d, PixelValue, Texture2dDataSource, MipmapsOption, UncompressedFloatFormat};
use std::borrow::Cow;
use std::rc::Rc;

use super::UniformsStorageVec;
use errors::*;
use font::{FreeTypeRasterizer, GlyphCache, GlyphLoader, RenderedGlyph};
use graphics::{Texture, GpuGlyph};

impl<'a, P> Texture2dDataSource<'a> for &'a Texture<P>
    where P: Clone + PixelValue
{
    type Data = P;

    fn into_raw(self) -> RawImage2d<'a, P> {
        RawImage2d {
            data: Cow::Borrowed(&self.data),
            width: self.width as u32,
            height: self.height as u32,
            format: <P as PixelValue>::get_format(),
        }
    }
}

struct GliumGpuGlyph {
    texture: Texture2d,
    glyph: RenderedGlyph,
}

impl<B> GpuGlyph<B> for GliumGpuGlyph where B: Facade + ?Sized {
    fn new(backend: &B, glyph: RenderedGlyph) -> Result<Self> {
        let texture = glyph.clone().into();
        Ok(Self {
            texture: Texture2d::with_format(backend, &texture, UncompressedFloatFormat::U8, MipmapsOption::NoMipmap)?,
            glyph: glyph,
        })
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);

pub struct TextRenderer {
    glyph_cache: GlyphCache<GliumGpuGlyph>,
    program: Program,
}

impl TextRenderer where {
    pub fn new<B>(facade: &B, font_path: &str, font_size: u32) -> Result<Self> where B: Facade {
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
        let mut advance = 0.0;
        for c in text.chars() {
            let glyph = self.glyph_cache.get(c as usize, facade)?;

            let (win_width, win_height) = surface.get_dimensions();
            let p_x = 1.0 / win_width as f32;
            let p_y = 1.0 / win_height as f32;

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
            uniforms.push("glyphTexture", glyph.texture.sampled());
            uniforms.push("projection", projection);

            let x = x + glyph.glyph.bearing_x as f32 + advance;
            let y = y - (glyph.glyph.height as f32 - glyph.glyph.bearing_y);

            let w = glyph.glyph.width as f32;
            let h = glyph.glyph.height as f32;

            #[cfg_attr(rustfmt, rustfmt_skip)]
            let vertices = [
                Vertex { position: [x    , y + h], tex_coords: [0.0, 0.0] },
                Vertex { position: [x    , y    ], tex_coords: [0.0, 1.0] },
                Vertex { position: [x + w, y    ], tex_coords: [1.0, 1.0] },
                Vertex { position: [x    , y + h], tex_coords: [0.0, 0.0] },
                Vertex { position: [x + w, y    ], tex_coords: [1.0, 1.0] },
                Vertex { position: [x + w, y + h], tex_coords: [1.0, 0.0] },
            ];

            let vertex_buffer = VertexBuffer::new(facade, &vertices)?;
            let index_buffer = NoIndices(PrimitiveType::TrianglesList);

            let params = DrawParameters {
                blend: Blend::alpha_blending(),
                ..Default::default()
            };

            surface.draw(&vertex_buffer, &index_buffer, &self.program, &uniforms, &params)?;

            advance += glyph.glyph.advance as f32;
        }

        Ok(())
    }
}
