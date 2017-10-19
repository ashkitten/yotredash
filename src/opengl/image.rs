use glium::backend::Facade;
use glium::texture::{RawImage2d, Texture2d};
use image::ImageFormat::*;
use image::{Frame};
use image::{self, ImageDecoder};
use std::fs::File;
use std::io::{BufReader, SeekFrom};
use std::io::prelude::*;
use std::path::Path;

use errors::*;

pub struct Image {
    texture: Texture2d,
    frames: Vec<Frame>,
}

impl Image {
    pub fn new(path: &Path, facade: &Facade) -> Result<Self> {
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
            PPM => image::ppm::PPMDecoder::new(buf_reader)?.into_frames()?,
            TIFF => image::tiff::TIFFDecoder::new(buf_reader)?.into_frames()?,
            TGA => image::tga::TGADecoder::new(buf_reader).into_frames()?,
            BMP => image::bmp::BMPDecoder::new(buf_reader).into_frames()?,
            ICO => image::ico::ICODecoder::new(buf_reader)?.into_frames()?,
            _ => bail!("Image format not supported"),
        }.collect::<Vec<Frame>>();

        // Assume the dimensions can't change... can they?
        let (width, height) = frames[0].buffer().dimensions();

        Ok(Self {
            texture: Texture2d::empty(facade, width, height)?,
            frames: frames,
        })
    }

    pub fn render_to_self(&mut self, facade: &Facade, time: f32) -> Result<()> {
        let frame = &self.frames.iter().cycle().nth(time as usize).unwrap();
        let buffer = frame.buffer().clone();
        let dimensions = buffer.dimensions();
        let image = RawImage2d::from_raw_rgba_reversed(&buffer.into_raw(), dimensions);
        self.texture = Texture2d::new(facade, image)?;

        Ok(())
    }

    pub fn texture(&self) -> &Texture2d {
        &self.texture
    }
}
