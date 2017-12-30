use std::path::Path;
use std::sync::Arc;
use time::Duration;
use vulkano::buffer::{BufferAccess, BufferUsage};
use vulkano::buffer::cpu_access::CpuAccessibleBuffer;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass};
use vulkano::instance::debug::DebugCallback;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::Swapchain;
use vulkano::sync::GpuFuture;
use vulkano_win::{VkSurfaceBuild, Window};
use winit::{EventsLoop, WindowBuilder};

use Renderer;
use config::Config;
use errors::*;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
}
impl_vertex!(Vertex, position);

mod vs {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[src = "
        #version 450
        layout(location = 0) in vec2 position;
        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    "]
    struct Dummy;
}

mod fs {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[src = "
        #version 450
        layout(location = 0) out vec4 color;
        void main() {
            color = vec4(1.0, 0.0, 0.0, 1.0);
        }
    "]
    struct Dummy;
}

pub struct VulkanRenderer {
    window: Window,
    device: Arc<Device>,
    framebuffers: Vec<Arc<FramebufferAbstract + Send + Sync>>,
    pipeline: Arc<GraphicsPipelineAbstract + Send + Sync>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    vertex_buffer: Arc<BufferAccess + Send + Sync>,
    _callback: Option<DebugCallback>,
}

impl Renderer for VulkanRenderer {
    fn new(config: Config, events_loop: &EventsLoop) -> Result<Self> {
        let layers = vec![
            #[cfg(debug_assertions)]
            "VK_LAYER_LUNARG_standard_validation",
        ];

        let instance = ::vulkano::instance::Instance::new(None, &::vulkano_win::required_extensions(), &layers)
            .expect("no instance with surface extension");

        let _callback = DebugCallback::errors_and_warnings(&instance, |msg| {
            println!("Debug callback: {:?}", msg.description);
        }).ok();

        let physical = ::vulkano::instance::PhysicalDevice::enumerate(&instance)
            .next()
            .expect("no graphics device");

        let window = WindowBuilder::new()
            .with_title("yotredash")
            .build_vk_surface(events_loop, instance.clone())?;

        let (device, mut queues) = {
            let graphical_queue_family = physical
                .queue_families()
                .find(|&q| q.supports_graphics() && window.surface().is_supported(q).unwrap_or(false))
                .expect("couldn't find a graphic queue family");
            let device_ext = DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::none()
            };
            Device::new(
                physical.clone(),
                physical.supported_features(),
                &device_ext,
                [(graphical_queue_family, 0.5)].iter().cloned(),
            ).expect("failed to create device")
        };

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let caps = window
                .surface()
                .capabilities(device.physical_device())
                .expect("failure to get surface capabilities");
            let format = caps.supported_formats[0].0;
            let dimensions = caps.current_extent.unwrap_or([1024, 768]);
            let usage = caps.supported_usage_flags;
            let present = caps.present_modes.iter().next().unwrap();

            Swapchain::new(
                device.clone(),
                window.surface().clone(),
                caps.min_image_count,
                format,
                dimensions,
                1,
                usage,
                &queue,
                ::vulkano::swapchain::SurfaceTransform::Identity,
                ::vulkano::swapchain::CompositeAlpha::Opaque,
                present,
                true,
                None,
            ).expect("failed to create swapchain")
        };

        let renderpass = Arc::new(single_pass_renderpass!(
                device.clone(), attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: swapchain.format(),
                        samples: 1,
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {}
                }
            )?);

        let vs = vs::Shader::load(device.clone()).expect("failed to create shader module");
        let fs = fs::Shader::load(device.clone()).expect("failed to create shader module");

        let pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input(SingleBufferDefinition::<Vertex>::new())
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports(
                [
                    Viewport {
                        origin: [0.0, 0.0],
                        depth_range: 0.0..1.0,
                        dimensions: [
                            images[0].dimensions()[0] as f32,
                            images[0].dimensions()[1] as f32,
                        ],
                    },
                ].iter()
                    .cloned(),
            )
            .fragment_shader(fs.main_entry_point(), ())
            .cull_mode_front()
            .front_face_counter_clockwise()
            .depth_stencil_disabled()
            .render_pass(Subpass::from(renderpass.clone(), 0).unwrap())
            .build(device.clone())?);

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            #[cfg_attr(rustfmt, rustfmt_skip)]
            [
                Vertex { position: [-1.0, -1.0] },
                Vertex { position: [ 1.0, -1.0] },
                Vertex { position: [ 1.0,  1.0] },
                Vertex { position: [-1.0, -1.0] },
                Vertex { position: [ 1.0,  1.0] },
                Vertex { position: [-1.0,  1.0] },
            ]
                .iter()
                .cloned(),
        ).expect("failed to create vertex buffer");

        // NOTE: We don't create any descriptor sets in this example, but you should
        // note that passing wrong types, providing sets at wrong indexes will cause
        // descriptor set builder to return Err!

        let framebuffers = images
            .iter()
            .map(|image| {
                Arc::new(
                    Framebuffer::start(renderpass.clone())
                        .add(image.clone())
                        .unwrap()
                        .build()
                        .unwrap(),
                ) as Arc<FramebufferAbstract + Send + Sync>
            })
            .collect();

        Ok(Self {
            window: window,
            device: device,
            framebuffers: framebuffers,
            pipeline: pipeline,
            queue: queue,
            swapchain: swapchain,
            vertex_buffer: vertex_buffer,
            _callback: _callback,
        })
    }

    fn render(&mut self, time: Duration, pointer: [f32; 4], fps: f32) -> Result<()> {
        let (image_num, acquire_future) = ::vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None)
            .expect("failed to acquire swapchain in time");

        let command_buffer = AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())?
            .begin_render_pass(
                self.framebuffers[image_num].clone(),
                false,
                vec![[0.0, 0.0, 0.0, 1.0].into(), 1.0.into()],
            )?
            .draw(
                self.pipeline.clone(),
                DynamicState::none(),
                vec![self.vertex_buffer.clone()],
                (),
                (),
            )?
            .end_render_pass()?
            .build()?;

        acquire_future
            .then_execute(self.queue.clone(), command_buffer)?
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
            .then_signal_fence_and_flush()?
            .wait(None)?;

        Ok(())
    }

    fn render_to_file(&mut self, time: Duration, pointer: [f32; 4], fps: f32, path: &Path) -> Result<()> {
        Ok(())
    }

    fn swap_buffers(&self) -> Result<()> {
        Ok(())
    }

    fn reload(&mut self, config: &Config) -> Result<()> {
        info!("Reloading config");
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        Ok(())
    }
}
