use glium::{Blend, DrawParameters, Program, Surface, Texture2d, VertexBuffer};
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, UncompressedFloatFormat};
use std::borrow::Cow;
use std::rc::Rc;

use super::UniformsStorageVec;
use errors::*;
use font::{FreeTypeRasterizer, GlyphCache, GlyphLoader};

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

impl TextRenderer {
    pub fn new(facade: &Facade, font_path: &str, font_size: u32) -> Result<Self> {
        let glyph_cache = GlyphCache::new(Rc::new(FreeTypeRasterizer::new(font_path, font_size)?))?;

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
            let glyph = self.glyph_cache.get(c as usize)?;

            let image = RawImage2d {
                data: Cow::from(glyph.buffer.clone()),
                width: glyph.width as u32,
                height: glyph.rows as u32,
                format: ClientFormat::U8,
            };
            let image = Texture2d::with_format(facade, image, UncompressedFloatFormat::U8, MipmapsOption::NoMipmap)?;

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
            uniforms.push("glyphTexture", image.sampled());
            uniforms.push("projection", projection);

            let x = x + glyph.bearing_x as f32 + advance;
            let y = y - (glyph.rows as f32 - glyph.bearing_y);

            let w = glyph.width as f32;
            let h = glyph.rows as f32;

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

            advance += glyph.advance as f32;
        }

        Ok(())
    }
}
