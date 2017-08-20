extern crate glium;

use glium::uniforms::{AsUniformValue, UniformValue, Uniforms};
use std::borrow::Cow;

pub struct UniformsStorageVec<'name, 'uniform>(Vec<(Cow<'name, str>, Box<AsUniformValue + 'uniform>)>);

impl<'name, 'uniform> UniformsStorageVec<'name, 'uniform> {
    pub fn new() -> Self {
        UniformsStorageVec(Vec::new())
    }

    pub fn push<S, U>(&mut self, name: S, uniform: U)
    where
        S: Into<Cow<'name, str>>,
        U: AsUniformValue + 'uniform,
    {
        self.0.push((name.into(), Box::new(uniform)))
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
