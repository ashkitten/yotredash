//! A `Source` that reads an image from file and returns frames from that image

use gif::{self, SetParameter};
use gif_dispose;
use image::{self, ImageDecoder};
use image::ImageFormat::*;
use std::fs::File;
use std::io::{BufReader, SeekFrom};
use std::io::prelude::*;
use std::path::Path;
use time::{self, Duration, Tm};

use super::{Frame, Source};
use errors::*;
use surface::Surface;

/// A `Source` that reads an image from file and returns frames from that image
pub struct ImageSource {
    /// The name of the source
    name: String,
    /// The time that the current frame started rendering - we need to keep track of this so we can
    /// increment the frame number when the delay is done
    frame_start: Tm,
    /// The current frame of an animated image
    current_frame: usize,
    /// A `Vec` of `Frame`s and their corresponding `Duration`s
    frames: Vec<(Frame, Duration)>,
}

impl Source for ImageSource {
    fn new(name: &str, path: &Path) -> Result<Self> {
        info!("New image source: {}", path.to_str().unwrap());

        let file = File::open(path).chain_err(|| "Could not open image file")?;
        let mut buf_reader = BufReader::new(file);
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        buf_reader.seek(SeekFrom::Start(0))?;

        fn decode_frame<D>(decoder: D) -> Result<(Frame, Duration)>
        where
            D: ImageDecoder,
        {
            let buffer = decoder.into_frames()?.nth(0).unwrap().into_buffer();
            let (width, height) = buffer.dimensions();
            let buffer = buffer.into_raw();
            Ok((
                Frame {
                    width,
                    height,
                    buffer,
                },
                Duration::zero(),
            ))
        }

        let format = image::guess_format(&buf)?;
        let frames = match format {
            BMP => vec![decode_frame(image::bmp::BMPDecoder::new(buf_reader))?],
            ICO => vec![decode_frame(image::ico::ICODecoder::new(buf_reader)?)?],
            JPEG => vec![decode_frame(image::jpeg::JPEGDecoder::new(buf_reader))?],
            PNG => vec![decode_frame(image::png::PNGDecoder::new(buf_reader))?],
            PNM => vec![decode_frame(image::pnm::PNMDecoder::new(buf_reader)?)?],
            TGA => vec![decode_frame(image::tga::TGADecoder::new(buf_reader))?],
            TIFF => vec![decode_frame(image::tiff::TIFFDecoder::new(buf_reader)?)?],
            WEBP => vec![decode_frame(image::webp::WebpDecoder::new(buf_reader))?],
            GIF => {
                let mut decoder = gif::Decoder::new(buf_reader);
                decoder.set(gif::ColorOutput::Indexed);
                let mut reader = decoder.read_info()?;
                let mut screen = gif_dispose::Screen::new_reader(&reader);

                let mut frames = Vec::new();
                while let Some(frame) = reader.read_next_frame()? {
                    screen.blit_frame(&frame)?;

                    let mut buffer = Vec::new();
                    for rgba in screen.pixels.pixels() {
                        buffer.extend(rgba.iter());
                    }

                    frames.push((
                        Frame {
                            width: screen.pixels.width() as u32,
                            height: screen.pixels.height() as u32,
                            buffer,
                        },
                        Duration::milliseconds(frame.delay as i64 * 10),
                    ));
                }
                frames
            }
            _ => bail!("Image format not supported"),
        };

        info!("Frame count: {}", frames.len());

        Ok(Self {
            name: name.to_string(),
            frame_start: time::now(),
            current_frame: 0,
            frames: frames,
        })
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn update(&mut self) -> bool {
        if self.frames.len() == 1 {
            return false;
        }

        if time::now() - self.frame_start > self.frames[self.current_frame].1 {
            self.current_frame += 1;
            if self.current_frame == self.frames.len() {
                self.current_frame = 0;
            }
            self.frame_start = time::now();
            return true;
        }

        false
    }

    fn get_frame(&self) -> Frame {
        self.frames[self.current_frame].0.clone()
    }

    fn write_frame(&self, surface: &mut Surface) -> Result<()> {
        let ref frame = self.frames[self.current_frame].0;
        surface.write_buffer(&frame.buffer, frame.dimensions())?;

        Ok(())
    }
}
