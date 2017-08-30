extern crate glium;
extern crate image;

use glium::Surface;
use std;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;
use std::rc::Rc;

use Vertex;
use config::Config;
use uniforms::UniformsStorageVec;

pub struct Buffer {
    texture: glium::texture::Texture2d,
    program: glium::Program,
    textures: Vec<glium::texture::Texture2d>,
    depends: Vec<Rc<Buffer>>,
}

impl Buffer {
    pub fn new(facade: &glium::backend::Facade, config: &Config, name: &str) -> Buffer {
        let file = match File::open(config.buffers[name].clone().vertex) {
            Ok(file) => file,
            Err(error) => {
                error!("Could not open vertex shader file: {}", error);
                std::process::exit(1);
            }
        };
        let mut buf_reader = BufReader::new(file);
        let mut vertex_source = String::new();
        match buf_reader.read_to_string(&mut vertex_source) {
            Ok(_) => info!("Using vertex shader: {}", config.buffers[name].vertex),
            Err(error) => {
                error!("Could not read vertex shader file: {}", error);
                std::process::exit(1);
            }
        };

        let file = match File::open(config.buffers[name].clone().fragment) {
            Ok(file) => file,
            Err(error) => {
                error!("Could not open fragment shader file: {}", error);
                std::process::exit(1);
            }
        };
        let mut buf_reader = BufReader::new(file);
        let mut fragment_source = String::new();
        match buf_reader.read_to_string(&mut fragment_source) {
            Ok(_) => info!("Using fragment shader: {}", config.buffers[name].fragment),
            Err(error) => {
                error!("Could not read fragment shader file: {}", error);
                std::process::exit(1);
            }
        };

        let input = glium::program::ProgramCreationInput::SourceCode {
            vertex_shader: &vertex_source,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: &fragment_source,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
        };
        let program = glium::Program::new(facade, input);
        let program = match program {
            Ok(program) => program,
            Err(error) => {
                error!("{}", error);
                std::process::exit(1);
            }
        };

        let textures = config.buffers[name]
            .textures
            .iter()
            .map(|name: &String| {
                let image = image::open(Path::new(&config.textures[name].path))
                    .unwrap()
                    .to_rgba();
                let image_dimensions = image.dimensions();
                let image = glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
                glium::texture::Texture2d::new(facade, image).unwrap()
            })
            .collect();

        let depends = config.buffers[name]
            .depends
            .iter()
            .map(|name: &String| Rc::new(Buffer::new(facade, config, name)))
            .collect();

        Buffer {
            texture: glium::texture::Texture2d::empty(facade, config.buffers[name].width, config.buffers[name].height)
                .unwrap(),
            program: program,
            textures: textures,
            depends: depends,
        }
    }

    pub fn render_to<S: Surface>(
        &self, target: &mut S, vertex_buffer: &glium::VertexBuffer<Vertex>, index_buffer: &glium::index::NoIndices,
        time: f32, pointer: (f32, f32, f32, f32)
    ) where
        S: Surface,
    {
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        let mut uniforms = UniformsStorageVec::new();
        uniforms.push("resolution", (self.texture.get_width() as f32, self.texture.get_height().unwrap() as f32));
        uniforms.push("time", time as f32);
        uniforms.push("pointer", (
            pointer.0,
            self.texture.get_height().unwrap() as f32 - pointer.1,
            pointer.2,
            self.texture.get_height().unwrap() as f32 - pointer.3,
        ));
        for (i, texture) in self.textures.iter().enumerate() {
            uniforms.push(format!("texture{}", i), texture);
        }
        for (i, buffer) in self.depends.iter().enumerate() {
            buffer.render_to_texture(vertex_buffer, index_buffer, time, pointer);
            uniforms.push(format!("buffer{}", i), buffer.texture.sampled());
        }

        target
            .draw(vertex_buffer, index_buffer, &self.program, &uniforms, &Default::default())
            .unwrap();
    }

    pub fn render_to_texture(
        &self, vertex_buffer: &glium::VertexBuffer<Vertex>, index_buffer: &glium::index::NoIndices, time: f32,
        pointer: (f32, f32, f32, f32)
    ) {
        self.render_to(&mut self.texture.as_surface(), vertex_buffer, index_buffer, time, pointer);
    }

    pub fn resize(&mut self, facade: &glium::backend::Facade, width: u32, height: u32) {
        self.texture = glium::texture::Texture2d::empty(facade, width, height).unwrap();
        for buffer in &mut self.depends {
            Rc::get_mut(buffer).unwrap().resize(facade, width, height);
        }
    }
}
