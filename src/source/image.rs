use image::{self, ImageDecoder};
use image::ImageFormat::*;
use num_rational::Ratio;
use std::fs::File;
use std::io::{BufReader, SeekFrom};
use std::io::prelude::*;
use std::path::Path;
use time::Duration;

use super::{Frame, Source};
use errors::*;

pub struct ImageSource {
    time_to_frame: Duration,
    current_frame: usize,
    frames: Vec<image::Frame>,
}

impl Source for ImageSource {
    fn new(path: &Path) -> Result<Self> {
        let file = File::open(path).chain_err(|| "Could not open image file")?;
        let mut buf_reader = BufReader::new(file);
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        buf_reader.seek(SeekFrom::Start(0))?;

        let format = image::guess_format(&buf)?;
        let frames = match format {
            PNG => image::png::PNGDecoder::new(buf_reader).into_frames()?,
            JPEG => image::jpeg::JPEGDecoder::new(buf_reader).into_frames()?,
            GIF => image::gif::Decoder::new(buf_reader).into_frames()?,
            WEBP => image::webp::WebpDecoder::new(buf_reader).into_frames()?,
            PNM => image::pnm::PNMDecoder::new(buf_reader)?.into_frames()?,
            TIFF => image::tiff::TIFFDecoder::new(buf_reader)?.into_frames()?,
            TGA => image::tga::TGADecoder::new(buf_reader).into_frames()?,
            BMP => image::bmp::BMPDecoder::new(buf_reader).into_frames()?,
            ICO => image::ico::ICODecoder::new(buf_reader)?.into_frames()?,
            _ => bail!("Image format not supported"),
        }.collect::<Vec<image::Frame>>();

        Ok(Self {
            time_to_frame: Duration::zero(),
            current_frame: 0,
            frames: frames,
        })
    }

    fn width(&self) -> u32 {
        self.frames[0].buffer().width()
    }

    fn height(&self) -> u32 {
        self.frames[0].buffer().height()
    }

    fn update(&mut self) -> bool {
        if self.frames.len() == 1 {
            return false;
        }

        // Delay in millis
        let delay = (self.frames[self.current_frame].delay() * Ratio::from_integer(1000)).to_integer();

        let mut ret = false;
        while self.time_to_frame.num_milliseconds() as f32 / 1000.0 > delay as f32 {
            self.time_to_frame = self.time_to_frame - Duration::milliseconds(delay as i64);
            self.current_frame += 1;
            ret = true;
        }
        ret
    }

    fn get_frame(&self) -> Frame {
        let buffer = self.frames[self.current_frame].buffer().clone();

        Frame {
            width: buffer.width() as u32,
            height: buffer.height() as u32,
            buffer: buffer.into_raw(),
        }
    }
}
