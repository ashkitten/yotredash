// Create the Error, ErrorKind, ResultExt, and Result types
#[derive(Debug, ErrorChain)]
pub enum ErrorKind {
    Msg(String),

    #[error_chain(foreign)] FreeTypeError(::freetype::Error),

    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumDisplayCreationError(::glium::backend::glutin::DisplayCreationError),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumDrawError(::glium::DrawError),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumGlutinCreationError(::glium::glutin::CreationError),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumIncompatibleOpenGlError(::glium::IncompatibleOpenGl),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumProgramChooserCreationError(::glium::program::ProgramChooserCreationError),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumProgramCreationError(::glium::ProgramCreationError),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumSwapBuffersError(::glium::SwapBuffersError),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumTextureCreationError(::glium::texture::TextureCreationError),
    #[cfg(feature = "opengl")]
    #[error_chain(foreign)]
    GliumVertexCreationError(::glium::vertex::BufferCreationError),

    #[cfg(feature = "image-src")]
    #[error_chain(foreign)]
    ImageError(::image::ImageError),
    #[cfg(feature = "image-src")]
    #[error_chain(foreign)]
    GifDecodingError(::gif::DecodingError),
    #[cfg(feature = "image-src")]
    #[error_chain(foreign)]
    GifDisposeError(::gif_dispose::Error),

    #[error_chain(foreign)] LogSetLoggerError(::log::SetLoggerError),

    #[error_chain(foreign)] NFDError(::nfd::error::NFDError),

    #[error_chain(foreign)] SerdeYamlError(::serde_yaml::Error),

    #[error_chain(foreign)] StdIoError(::std::io::Error),
    #[error_chain(foreign)] StdParseIntError(::std::num::ParseIntError),
    #[error_chain(foreign)] StdParseFloatError(::std::num::ParseFloatError),

    #[error_chain(foreign)] TimeParseError(::time::ParseError),

    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoAutoCommandBufferBuilderContextError(::vulkano::command_buffer::AutoCommandBufferBuilderContextError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoBeginRenderPassError(::vulkano::command_buffer::BeginRenderPassError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoBuildError(::vulkano::command_buffer::BuildError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoCommandBufferExecError(::vulkano::command_buffer::CommandBufferExecError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoDrawError(::vulkano::command_buffer::DrawError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoFlushError(::vulkano::sync::FlushError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoGraphicsPipelineCreationError(::vulkano::pipeline::GraphicsPipelineCreationError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoOomError(::vulkano::OomError),
    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoRenderPassCreationError(::vulkano::framebuffer::RenderPassCreationError),

    #[cfg(feature = "vulkan")]
    #[error_chain(foreign)]
    VulkanoWinCreationError(::vulkano_win::CreationError),
}
