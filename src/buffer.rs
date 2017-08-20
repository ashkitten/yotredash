extern crate glium;
extern crate image;
extern crate json;

// Glium

use glium::Surface;

// Std

use std;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

// Local

use Vertex;
use uniforms::UniformsStorageVec;

// Structs

pub struct Buffer {
    texture: glium::texture::Texture2d,
    program: glium::Program,
    uniform_textures: Vec<glium::texture::Texture2d>,
    uniform_buffers: Vec<Buffer>,
}

impl Buffer {
    pub fn new(facade: &glium::backend::Facade, info: &json::JsonValue, base_dir: &PathBuf) -> Buffer {
        let file = match File::open(base_dir.join(Path::new(info["vertex"].as_str().unwrap_or_default()))) {
            Ok(file) => file,
            Err(error) => {
                error!("Could not open vertex shader file: {}", error);
                std::process::exit(1);
            }
        };
        let mut buf_reader = BufReader::new(file);
        let mut vertex_source = String::new();
        match buf_reader.read_to_string(&mut vertex_source) {
            Ok(_) => info!("Using vertex shader: {}", info["vertex"]),
            Err(error) => {
                error!("Could not read vertex shader file: {}", error);
                std::process::exit(1);
            }
        };

        let file = match File::open(base_dir.join(Path::new(info["fragment"].as_str().unwrap_or_default()))) {
            Ok(file) => file,
            Err(error) => {
                error!("Could not open fragment shader file: {}", error);
                std::process::exit(1);
            }
        };
        let mut buf_reader = BufReader::new(file);
        let mut fragment_source = String::new();
        match buf_reader.read_to_string(&mut fragment_source) {
            Ok(_) => info!("Using fragment shader: {}", info["fragment"]),
            Err(error) => {
                error!("Could not read fragment shader file: {}", error);
                std::process::exit(1);
            }
        };

        let program = glium::Program::from_source(facade, &vertex_source, &fragment_source, None);
        let program = match program {
            Ok(program) => program,
            Err(error) => {
                error!("{}", error);
                std::process::exit(1);
            }
        };

        let uniform_textures = info["textures"]
            .members()
            .map(|path: &json::JsonValue| {
                let path = base_dir.join(Path::new(path.as_str().unwrap()));
                let image = image::open(path).unwrap().to_rgba();
                let image_dimensions = image.dimensions();
                let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
                glium::texture::Texture2d::new(facade, image).unwrap()
            })
            .collect();

        let uniform_buffers = info["buffers"]
            .members()
            .map(|info: &json::JsonValue| Buffer::new(facade, info, base_dir))
            .collect();

        return Buffer {
            texture: glium::texture::Texture2d::empty(
                facade,
                info["width"].as_i64().unwrap_or(640) as u32,
                info["height"].as_i64().unwrap_or(400) as u32,
            ).unwrap(),
            program: program,
            uniform_textures: uniform_textures,
            uniform_buffers: uniform_buffers,
        };
    }

    pub fn render_to<S: Surface>(
        &self, target: &mut S, vertex_buffer: &glium::VertexBuffer<Vertex>, index_buffer: &glium::index::NoIndices,
        time: f32, pointer: (f32, f32, f32, f32),
    ) where
        S: Surface,
    {
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        let mut uniforms = UniformsStorageVec::new();
        uniforms.push("resolution", (self.texture.get_width() as f32, self.texture.get_height().unwrap() as f32));
        uniforms.push("time", time as f32);
        uniforms.push(
            "pointer",
            (
                pointer.0,
                self.texture.get_height().unwrap() as f32 - pointer.1,
                pointer.2,
                self.texture.get_height().unwrap() as f32 - pointer.3,
            ),
        );
        for (i, texture) in self.uniform_textures.iter().enumerate() {
            uniforms.push(format!("texture{}", i), texture);
        }
        for (i, buffer) in self.uniform_buffers.iter().enumerate() {
            buffer.render_to_texture(vertex_buffer, index_buffer, time, pointer);
            uniforms.push(format!("buffer{}", i), buffer.texture.sampled());
        }

        target
            .draw(vertex_buffer, index_buffer, &self.program, &uniforms, &Default::default())
            .unwrap();
    }

    pub fn render_to_texture(
        &self, vertex_buffer: &glium::VertexBuffer<Vertex>, index_buffer: &glium::index::NoIndices, time: f32,
        pointer: (f32, f32, f32, f32),
    ) {
        self.render_to(&mut self.texture.as_surface(), vertex_buffer, index_buffer, time, pointer);
    }

    pub fn resize(&mut self, facade: &glium::backend::Facade, width: u32, height: u32) {
        self.texture = glium::texture::Texture2d::empty(facade, width, height).unwrap();
        for mut buffer in self.uniform_buffers.iter_mut() {
            buffer.resize(facade, width, height);
        }
    }
}
