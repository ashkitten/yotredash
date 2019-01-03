//! A `Node` that reads an image from file and returns frames from that image

use failure::{bail, Error, ResultExt};
use gif::{self, SetParameter};
use gif_dispose;
use glium::{
    backend::Facade,
    texture::{MipmapsOption, RawImage2d, Texture2d},
};
use image::{self, ImageDecoder, ImageFormat::*};
use log::debug;
use std::{
    collections::HashMap,
    fs::File,
    io::{prelude::*, BufReader, SeekFrom},
    rc::Rc,
};
use time::{self, Duration, Tm};

use super::{Node, NodeInputs, NodeOutput};
use crate::config::nodes::ImageConfig;

/// A `Node` that reads an image from file and returns frames from that image
pub struct ImageNode {
    /// GPU texture containing an atlas of the image frames
    textures: Vec<Rc<Texture2d>>,
    /// The current frame of an animated image
    current_frame: usize,
    /// The time that the current frame started rendering - we need to keep track of this so we can
    /// increment the frame number when the delay is done
    frame_start: Tm,
    /// Array of frame durations
    durations: Vec<Duration>,
}

impl ImageNode {
    /// Create a new instance
    pub fn new(facade: &Rc<dyn Facade>, config: ImageConfig) -> Result<Self, Error> {
        debug!("New image node: {}", config.path.to_string_lossy());

        let file = File::open(config.path).context("Could not open image file")?;
        let mut buf_reader = BufReader::new(file);
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        buf_reader.seek(SeekFrom::Start(0))?;

        fn decode_single<D>(facade: &Rc<dyn Facade>, decoder: D) -> Result<ImageNode, Error>
        where
            D: ImageDecoder,
        {
            let buffer = decoder.into_frames()?.nth(0).unwrap().into_buffer();
            let (width, height) = buffer.dimensions();
            let buffer = buffer.into_raw();
            let raw = RawImage2d::from_raw_rgba_reversed(&buffer, (width, height));
            let textures = vec![Rc::new(Texture2d::with_mipmaps(
                &**facade,
                raw,
                MipmapsOption::NoMipmap,
            )?)];

            Ok(ImageNode {
                textures,
                current_frame: 0,
                frame_start: time::now(),
                durations: Vec::new(),
            })
        }

        let format = image::guess_format(&buf)?;
        Ok(match format {
            BMP => decode_single(facade, image::bmp::BMPDecoder::new(buf_reader))?,
            ICO => decode_single(facade, image::ico::ICODecoder::new(buf_reader)?)?,
            JPEG => decode_single(facade, image::jpeg::JPEGDecoder::new(buf_reader))?,
            PNG => decode_single(facade, image::png::PNGDecoder::new(buf_reader))?,
            PNM => decode_single(facade, image::pnm::PNMDecoder::new(buf_reader)?)?,
            TGA => decode_single(facade, image::tga::TGADecoder::new(buf_reader))?,
            TIFF => decode_single(facade, image::tiff::TIFFDecoder::new(buf_reader)?)?,
            WEBP => decode_single(facade, image::webp::WebpDecoder::new(buf_reader))?,
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

                let textures = raws
                    .into_iter()
                    .map(|raw| {
                        Rc::new(
                            Texture2d::with_mipmaps(&**facade, raw, MipmapsOption::NoMipmap)
                                .unwrap(),
                        )
                    })
                    .collect();

                Self {
                    textures,
                    current_frame: 0,
                    frame_start: time::now(),
                    durations,
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
    fn render(&mut self, _inputs: &NodeInputs) -> Result<HashMap<String, NodeOutput>, Error> {
        self.update();

        let mut outputs = HashMap::new();
        outputs.insert(
            "texture".to_string(),
            NodeOutput::Texture2d(Rc::clone(&self.textures[self.current_frame])),
        );
        Ok(outputs)
    }
}
