//! Contains everything for the OpenGL renderer pipeline

pub mod nodes;
pub mod renderer;

use glium::uniforms::{AsUniformValue, UniformValue, Uniforms};
use std::borrow::Cow;

/// Implementation of the vertex attributes for the vertex buffer
#[derive(Copy, Clone)]
pub struct Vertex {
    /// Position of the vertex in 2D space
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

/// A `UniformsStorage` which has a `push` method for appending new uniforms
#[derive(Clone, Default)]
pub struct UniformsStorageVec<'name, 'uniform>(
    Vec<(Cow<'name, str>, SelfAsUniformValue<'uniform>)>,
);

impl<'name, 'uniform> UniformsStorageVec<'name, 'uniform> {
    /// Create a new instance
    pub fn new() -> Self {
        Default::default()
    }

    /// Push a new uniform onto the array
    pub fn push<S, U>(&mut self, name: S, uniform: U)
    where
        S: Into<Cow<'name, str>>,
        U: AsUniformValue + 'uniform,
    {
        self.0.push((name.into(), SelfAsUniformValue(uniform.as_uniform_value())))
    }
}

impl<'name, 'uniform> Uniforms for UniformsStorageVec<'name, 'uniform> {
    #[inline]
    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, mut output: F) {
        for &(ref name, ref uniform) in &self.0 {
            output(name, uniform.as_uniform_value());
        }
    }
}

#[derive(Clone)]
pub struct SelfAsUniformValue<'a>(UniformValue<'a>);

impl<'a> AsUniformValue for SelfAsUniformValue<'a> {
    fn as_uniform_value(&self) -> UniformValue {
        self.0
    }
}
