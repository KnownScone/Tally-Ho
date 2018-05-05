extern crate chrono;
extern crate fern;
#[macro_use]
extern crate log;
#[macro_use]
extern crate vulkano;
extern crate vulkano_win;
extern crate winit;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate cgmath;

mod game;
mod scene;

use std::sync::Arc;
use std::cmp::{max, min};

use vulkano::swapchain::{Surface, Swapchain, CompositeAlpha, PresentMode};
use vulkano::swapchain;
use vulkano::buffer::{BufferUsage, CpuBufferPool, CpuAccessibleBuffer, DeviceLocalBuffer};
use vulkano::device::{Device};
use vulkano::image::{ImageUsage};
use vulkano::sync::{now, GpuFuture, SharingMode};
use vulkano::instance::{Features, Instance, InstanceExtensions, DeviceExtensions, PhysicalDevice};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::pipeline::{GraphicsPipeline};
use vulkano::pipeline::viewport::{Viewport};
use vulkano::framebuffer::{Framebuffer, Subpass};

use cgmath::prelude::*;
use cgmath::{ortho, Rad, Matrix4};

use winit::{EventsLoop, WindowBuilder, Window};

use vulkano_win::VkSurfaceBuild;

fn main() {
    // TODO: Handle this better, rather than just a panic
    init_logging().unwrap();

    info!("Logging initialized");

    let instance = {
        let extensions = vulkano_win::required_extensions();
        let info = app_info_from_cargo_toml!();

        Instance::new(
            Some(&info), 
            &extensions, 
            None
        ).expect("Couldn't build instance")
    };

    let physical_device = {
        PhysicalDevice::from_index(
            &instance, 
            0
        ).expect("No physical device")
    };

    let (device, mut queues) = {
        let queue_family = physical_device.queue_families();
        let features = Features::none();
        let extensions = DeviceExtensions {
            khr_swapchain: true,
            .. DeviceExtensions::none()
        };

        Device::new(
            physical_device, 
            &physical_device.supported_features(), 
            &extensions, 
            queue_family.map(|queue| (queue, 1.0))
        ).expect("Couldn't build device")
    };

    let mut events_loop = EventsLoop::new();

    let surface = WindowBuilder::new()
        .with_title("Tally Ho")
        .build_vk_surface(&events_loop, instance.clone())
        .expect("Couldn't build Vulkan surface");

    info!("Window initialized");

    let capabs = surface.capabilities(physical_device)
        .expect("Couldn't acquire surface capabilities");

    let mut dimensions = capabs.current_extent.unwrap_or([640, 480]);

    let (mut swapchain, mut images) = {
        // Try to use double-buffering.
        let buffers_count = max(min(2, capabs.min_image_count), capabs.max_image_count.unwrap_or(2));

        let transform = capabs.current_transform;

        let (format, color_space) = capabs.supported_formats[0];

        let usage = ImageUsage {
            color_attachment: true,
            .. ImageUsage::none()
        };

        let alpha = capabs.supported_composite_alpha.iter().next().unwrap();

        let sharing_mode = SharingMode::Exclusive(0);

        Swapchain::new(
            device.clone(),
            surface.clone(),
            buffers_count,
            format,
            dimensions,
            1,
            capabs.supported_usage_flags,
            sharing_mode,
            transform,
            alpha,
            PresentMode::Fifo,
            true,
            None
        ).expect("Couldn't build swapchain")
    };

    info!("Swapchain initialized");
    
    #[derive(Debug, Clone)]
    struct Vertex { position: [f32; 2] }
    impl_vertex!(Vertex, position);
    
    let vertex_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(), 
        BufferUsage::all(), 
        [
            Vertex { position: [-0.5, -0.25] },
            Vertex { position: [0.0, 0.5] },
            Vertex { position: [0.25, -0.1] }
        ].iter().cloned()
    ).expect("Couldn't create vertex buffer");

    info!("Vertex buffer initialized");

    mod vs {
        #[derive(VulkanoShader)]
        #[ty = "vertex"]
        #[src = "
#version 450
layout(location = 0) in vec2 position;

layout(set = 0, binding = 0) uniform Data {
    mat4 view;
    mat4 proj;
} uniforms;

void main() {
    gl_Position = uniforms.proj * uniforms.view * vec4(position, 0.0, 1.0);
}
"]
        struct Dummy;
    }

    mod fs {
        #[derive(VulkanoShader)]
        #[ty = "fragment"]
        #[src = "
#version 450
layout(location = 0) out vec4 f_color;
void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
}
"]
        struct Dummy;
    }
    
    let vs = vs::Shader::load(device.clone()).expect("Couldn't create shader module");
    let fs = fs::Shader::load(device.clone()).expect("Couldn't create shader module");

    info!("Shaders initialized");

    let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
        attachments: {
            // `color` is a custom name we give to the first and only attachment.
            color: {
                // `load: Clear` means that we ask the GPU to clear the content of this
                // attachment at the start of the drawing.
                load: Clear,
                // `store: Store` means that we ask the GPU to store the output of the draw
                // in the actual image. We could also ask it to discard the result.
                store: Store,
                // `format: <ty>` indicates the type of the format of the image. This has to
                // be one of the types of the `vulkano::format` module (or alternatively one
                // of your structs that implements the `FormatDesc` trait). Here we use the
                // generic `vulkano::format::Format` enum because we don't know the format in
                // advance.
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            // We use the attachment named `color` as the one and only color attachment.
            color: [color],
            // No depth-stencil attachment is indicated with empty brackets.
            depth_stencil: {}
        }
    ).unwrap());

    let pipeline = Arc::new(GraphicsPipeline::start()
        .vertex_input_single_buffer()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap());

    let mut framebuffers: Option<Vec<Arc<Framebuffer<_,_>>>> = None;

    // TODO: Don't use just this one queue in the future 
    let queue = queues.nth(0).unwrap();

    let mut proj = get_projection(dimensions);
    
    let mut view = Matrix4::look_at(cgmath::Point3::new(0.0, 0.0, 1.0), cgmath::Point3::new(0.0, 0.0, 0.0), cgmath::Vector3::new(0.0, -1.0, 0.0));

    let uniform_buffer = CpuBufferPool::<vs::ty::Data>::new(
        device.clone(),
        BufferUsage::uniform_buffer() | BufferUsage::transfer_source(),
    );

    let local_uniform_buffer = DeviceLocalBuffer::<vs::ty::Data>::new(
        device.clone(),
        BufferUsage::uniform_buffer_transfer_destination(),
        vec![queue.family()]
    ).expect("Couldn't create uniform device local buffer");
    
    info!("Uniform buffer initialized");

    let mut set = Arc::new(vulkano::descriptor::descriptor_set::PersistentDescriptorSet::start(pipeline.clone(), 0)
        .add_buffer(local_uniform_buffer.clone()).unwrap()
        .build().unwrap()
    );

    let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;
    let mut recreate_swapchain = false;
    let mut running = true;
    while running {
        previous_frame_end.cleanup_finished();

        if recreate_swapchain {
            dimensions = surface.capabilities(physical_device)
                .expect("Couldn't acquire surface capabilities")
                .current_extent.unwrap_or([640, 480]);

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                Err(swapchain::SwapchainCreationError::UnsupportedDimensions) => {
                    continue;
                },
                Err(err) => panic!("{:?}", err)
            };

            std::mem::replace(&mut swapchain, new_swapchain);
            std::mem::replace(&mut images, new_images);

            proj = get_projection(dimensions);
            
            framebuffers = None;
            recreate_swapchain = false;
        }

        if framebuffers.is_none() {
            let new_framebuffers = Some(images.iter().map(|image| {
                Arc::new(Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap())
            }).collect::<Vec<_>>());
            std::mem::replace(&mut framebuffers, new_framebuffers);
        }
        
        let uniform_data = vs::ty::Data {
            view: view.into(),
            proj: proj.into(),
        };

        let uniform_subbuffer = uniform_buffer.next(uniform_data).expect("Couldn't build uniform sub-buffer");

        let (image_index, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
            Ok(r) => r,
            Err(swapchain::AcquireError::OutOfDate) => { 
                recreate_swapchain = true; 
                continue; 
            },
            Err(err) => panic!("{:?}", err)
        };

        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
            .copy_buffer(
                uniform_subbuffer,
                local_uniform_buffer.clone()
            ).unwrap()

            .begin_render_pass(
                framebuffers.as_ref().unwrap()[image_index].clone(), 
                false, vec![[0.0, 0.0, 1.0, 1.0].into()]
            ).unwrap()

            .draw(pipeline.clone(),
                DynamicState {
                    line_width: None,
                    viewports: Some(vec![Viewport {
                        origin: [0.0, 0.0],
                        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                        depth_range: 0.0 .. 1.0,
                    }]),
                    scissors: None,
                },
                vertex_buffer.clone(), set.clone(), ()
            ).unwrap()
            
            .end_render_pass().unwrap()
            .build().unwrap();

        let future = previous_frame_end.join(acquire_future)
            .then_execute(queue.clone(), command_buffer).unwrap()
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_index)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(vulkano::sync::FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(vulkano::sync::now(device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(vulkano::sync::now(device.clone())) as Box<_>;
            }
        }

        events_loop.poll_events(|ev| {
            match ev {
                winit::Event::WindowEvent { event: winit::WindowEvent::Closed, .. } => { 
                    info!("Window closing");
                    running = false
                },
                _ => ()
            }
        });
    }
}

fn init_logging() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {}: {}",
                chrono::Local::now().format("[%y-%m-%d][%H:%M:%S]"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;

    Ok(())
}

fn get_projection(dimensions: [u32; 2]) -> Matrix4<f32> {
    let aspect = dimensions[0] as f32 / dimensions[1] as f32;
    let (w, h) = (1. * aspect, 1.);

    ortho(-w, w, -h, h, -10., 10.)
}