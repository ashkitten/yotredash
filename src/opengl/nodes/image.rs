//! A `Node` that reads an image from file and returns frames from that image

use failure::Error;
use failure::ResultExt;
use gif::{self, SetParameter};
use gif_dispose;
use glium::{Program, Surface, VertexBuffer};
use glium::backend::Facade;
use glium::index::{NoIndices, PrimitiveType};
use glium::program::ProgramCreationInput;
use glium::texture::{MipmapsOption, RawImage2d, Texture2d};
use image::ImageFormat::*;
use image::{self, ImageDecoder};
use owning_ref::OwningHandle;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, SeekFrom};
use std::path::Path;
use std::rc::Rc;
use time::{self, Duration, Tm};

use opengl::{MapAsUniform, UniformsStorageVec, Vertex};
use super::Node;
use util::DerefInner;

#[cfg_attr(rustfmt, rustfmt_skip)]
const VERTICES: [Vertex; 6] = [
    Vertex { position: [-1.0, -1.0] },
    Vertex { position: [ 1.0, -1.0] },
    Vertex { position: [ 1.0,  1.0] },
    Vertex { position: [-1.0, -1.0] },
    Vertex { position: [ 1.0,  1.0] },
    Vertex { position: [-1.0,  1.0] },
];

const VERTEX: &str = "
    #version 140

    in vec2 position;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
    }
";

const FRAGMENT: &str = "
    #version 140

    out vec4 color;

    uniform vec2 resolution;
    uniform sampler2D frame;

    void main() {
        vec2 uv = gl_FragCoord.xy / resolution;
        color = texture(frame, uv);
    }
";

/// A `Node` that reads an image from file and returns frames from that image
pub struct ImageNode {
    /// The name of the node
    name: String,
    /// The Facade used to create textures
    facade: Rc<Facade>,
    /// GPU texture containing an atlas of the image frames
    textures: Rc<Vec<Texture2d>>,
    /// The current frame of an animated image
    current_frame: usize,
    /// The time that the current frame started rendering - we need to keep track of this so we can
    /// increment the frame number when the delay is done
    frame_start: Tm,
    /// Array of frame durations
    durations: Vec<Duration>,
    /// Shader program used to render the image to the default framebuffer
    /// We have to use this to work around sRGB issues
    program: Program,
    /// Vertex buffer for shader
    vertex_buffer: VertexBuffer<Vertex>,
    /// Index buffer for shader
    index_buffer: NoIndices,
}

impl ImageNode {
    /// Create a new instance
    pub fn new(facade: &Rc<Facade>, name: String, path: &Path) -> Result<Self, Error> {
        debug!("New image node: {}", path.to_string_lossy());

        let file = File::open(path).context("Could not open image file")?;
        let mut buf_reader = BufReader::new(file);
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        buf_reader.seek(SeekFrom::Start(0))?;

        fn compile_program(facade: &Rc<Facade>) -> Result<Program, Error> {
            let input = ProgramCreationInput::SourceCode {
                vertex_shader: &VERTEX,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                geometry_shader: None,
                fragment_shader: &FRAGMENT,
                transform_feedback_varyings: None,
                outputs_srgb: true,
                uses_point_size: false,
            };

            Ok(Program::new(&**facade, input)?)
        }

        fn decode_single<D>(
            facade: &Rc<Facade>,
            name: String,
            decoder: D,
        ) -> Result<ImageNode, Error>
        where
            D: ImageDecoder,
        {
            let buffer = decoder.into_frames()?.nth(0).unwrap().into_buffer();
            let (width, height) = buffer.dimensions();
            let buffer = buffer.into_raw();
            let raw = RawImage2d::from_raw_rgba_reversed(&buffer, (width, height));
            let textures = Rc::new(vec![
                Texture2d::with_mipmaps(&**facade, raw, MipmapsOption::NoMipmap)?,
            ]);

            Ok(ImageNode {
                name,
                facade: Rc::clone(facade),
                textures,
                current_frame: 0,
                frame_start: time::now(),
                durations: Vec::new(),
                program: compile_program(facade)?,
                vertex_buffer: VertexBuffer::new(&**facade, &VERTICES)?,
                index_buffer: NoIndices(PrimitiveType::TrianglesList),
            })
        }

        let format = image::guess_format(&buf)?;
        Ok(match format {
            BMP => decode_single(facade, name, image::bmp::BMPDecoder::new(buf_reader))?,
            ICO => decode_single(facade, name, image::ico::ICODecoder::new(buf_reader)?)?,
            JPEG => decode_single(facade, name, image::jpeg::JPEGDecoder::new(buf_reader))?,
            PNG => decode_single(facade, name, image::png::PNGDecoder::new(buf_reader))?,
            PNM => decode_single(facade, name, image::pnm::PNMDecoder::new(buf_reader)?)?,
            TGA => decode_single(facade, name, image::tga::TGADecoder::new(buf_reader))?,
            TIFF => decode_single(facade, name, image::tiff::TIFFDecoder::new(buf_reader)?)?,
            WEBP => decode_single(facade, name, image::webp::WebpDecoder::new(buf_reader))?,
            GIF => {
                let mut decoder = gif::Decoder::new(buf_reader);
                decoder.set(gif::ColorOutput::Indexed);
                let mut reader = decoder.read_info()?;
                let mut screen = gif_dispose::Screen::new_reader(&reader);
                let width = reader.width() as usize;
                let height = reader.height() as usize;

                let mut raws = Vec::new();
                let mut durations = Vec::new();
                while let Some(frame) = reader.read_next_frame()? {
                    screen.blit_frame(frame)?;

                    let mut pixels = Vec::with_capacity(width * height);
                    for pixel in screen.pixels.pixels() {
                        pixels.extend(pixel.iter());
                    }
                    raws.push(RawImage2d::from_raw_rgba_reversed(
                        &pixels,
                        (width as u32, height as u32),
                    ));

                    // GIF delays are in 100ths of a second
                    durations.push(Duration::milliseconds(i64::from(frame.delay) * 10));
                }

                let textures = Rc::new(
                    raws.into_iter()
                        .map(|raw| {
                            Texture2d::with_mipmaps(&**facade, raw, MipmapsOption::NoMipmap)
                                .unwrap()
                        })
                        .collect(),
                );

                Self {
                    name,
                    facade: facade.clone(),
                    textures,
                    current_frame: 0,
                    frame_start: time::now(),
                    durations,
                    program: compile_program(facade)?,
                    vertex_buffer: VertexBuffer::new(&**facade, &VERTICES)?,
                    index_buffer: NoIndices(PrimitiveType::TrianglesList),
                }
            }
            _ => bail!("Image format not supported"),
        })
    }

    fn update(&mut self) {
        if self.textures.len() > 1
            && time::now() - self.frame_start > self.durations[self.current_frame]
        {
            self.current_frame += 1;
            if self.current_frame == self.textures.len() {
                self.current_frame = 0;
            }
            self.frame_start = time::now();
        }
    }
}

impl Node for ImageNode {
    fn render(&mut self, uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        self.update();

        let sampled = OwningHandle::new_with_fn(self.textures.clone(), |t| unsafe {
            DerefInner((*t)[self.current_frame].sampled())
        });
        let sampled = MapAsUniform(sampled, |s| &**s);

        uniforms.push(self.name.clone(), sampled);

        Ok(())
    }

    fn present(&mut self, _uniforms: &mut UniformsStorageVec) -> Result<(), Error> {
        self.update();

        let input = uniform! {
            resolution: {
                let (width, height) = self.facade.get_context().get_framebuffer_dimensions();
                (width as f32, height as f32)
            },
            frame: self.textures[self.current_frame].sampled(),
        };

        let mut target = self.facade.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &input,
            &Default::default(),
        ).unwrap(); // For some reason if we return this error, it panicks because finish() is never called
        target.finish()?;

        Ok(())
    }

    fn render_to_file(
        &mut self,
        _uniforms: &mut UniformsStorageVec,
        path: &Path,
    ) -> Result<(), Error> {
        self.update();

        let raw: RawImage2d<u8> = self.textures[self.current_frame].read();
        image::save_buffer(path, &raw.data, raw.width, raw.height, ::image::RGBA(8))?;

        Ok(())
    }
}
