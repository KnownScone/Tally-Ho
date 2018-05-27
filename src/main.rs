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
extern crate specs;
extern crate rlua;

mod resource;
mod component;
mod system;
mod script;
mod game;

use resource as res;
use component as comp;
use system as sys;

use std::sync::Arc;
use std::cmp::{max, min};

use vulkano as vk;
use vk::instance::{Instance, PhysicalDevice};
use vk::swapchain::{Swapchain, Surface};
use vk::framebuffer::{Framebuffer, Subpass};
use vk::command_buffer::{AutoCommandBufferBuilder};
use vk::buffer::{CpuBufferPool, DeviceLocalBuffer};
use vk::sync::{now, GpuFuture};
use vk::device::{Device};

use cgmath::prelude::*;
use cgmath::{ortho, Rad, Matrix4, Vector3};

use winit::{EventsLoop, WindowBuilder, Window};

use vulkano_win::VkSurfaceBuild;

mod vs {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[src = "
#version 450
layout(location = 0) in vec2 position;

layout(set = 0, binding = 0) uniform Instance {
    mat4 transform;
} instance;

void main() {
    gl_Position = instance.transform * vec4(position, 0.0, 1.0);
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

#[derive(Debug, Clone)]
pub struct Vertex { 
    position: [f32; 2],
}

impl_vertex!(Vertex, position);

pub fn select_physical_device<'a>(instance: &'a Arc<Instance>) -> Option<PhysicalDevice<'a>> {
    // TODO: Better physical device selection
    PhysicalDevice::from_index(
        instance, 
        0
    )
}

pub fn init_device(
    extensions: vk::instance::DeviceExtensions, 
    features: vulkano::instance::Features, 
    physical_device: PhysicalDevice
) -> Result<(Arc<Device>, vk::device::QueuesIter), vk::device::DeviceCreationError> {
    let queue_family = physical_device.queue_families();

    // TODO: Better handling of queue_family
    Device::new(
        physical_device, 
        &physical_device.supported_features(), 
        &extensions, 
        queue_family.map(|queue| (queue, 1.0))
    )
}

pub fn init_swapchain<W>(
    device: Arc<Device>,
    surface: Arc<Surface<W>>, 
    capabs: vk::swapchain::Capabilities,
    dimensions: [u32; 2]
) -> Result<(Arc<Swapchain<W>>, Vec<Arc<vk::image::SwapchainImage<W>>>), vk::swapchain::SwapchainCreationError> {
    // TODO: Comments on all these swapchain components

    // Try to use double-buffering.
    let buffers_count = max(min(2, capabs.min_image_count), capabs.max_image_count.unwrap_or(2));

    let transform = capabs.current_transform;

    let (format, color_space) = capabs.supported_formats[0];

    let usage = vk::image::ImageUsage {
        color_attachment: true,
        .. vk::image::ImageUsage::none()
    };

    let alpha = capabs.supported_composite_alpha.iter().next().unwrap();

    let sharing_mode = vk::sync::SharingMode::Exclusive(0);

    let present_mode = vk::swapchain::PresentMode::Fifo;

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
        present_mode,
        true,
        None
    )
}

pub fn init_render_pass<W>(
    device: Arc<Device>, 
    swapchain: Arc<Swapchain<W>>
) -> Result<Arc<vk::framebuffer::RenderPassAbstract + Send + Sync>, vk::framebuffer::RenderPassCreationError> {
    single_pass_renderpass!(device.clone(),
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
    ).map(|x| Arc::new(x) as Arc<vk::framebuffer::RenderPassAbstract + Send + Sync>)
}

// TODO: Texture rendering!

fn main() {
    // TODO: Handle this better, rather than just a panic
    init_logging().unwrap();

    info!("Logging initialized");

    let app_info = app_info_from_cargo_toml!();

    let instance = {
        let extensions = vulkano_win::required_extensions();

        Instance::new(
            Some(&app_info), 
            &extensions, 
            None
        ).expect("Couldn't initialize instance")
    };
    info!("Instance initialized");

    let physical_device = select_physical_device(&instance)
        .expect("Couldn't select physical device");
    info!("Physical device selected");

    let (device, mut queues) =
        init_device(
            vk::instance::DeviceExtensions {
                khr_swapchain: true,
                .. vk::instance::DeviceExtensions::none()
            }, 
            vk::instance::Features::none(), 
            physical_device
        ).expect("Couldn't initialize device");

    info!("Device initialized");
    info!("Queues initialized");

    let mut events_loop = EventsLoop::new();
    info!("Events loop initialized");

    let surface = WindowBuilder::new()
        .with_title(app_info.application_name.unwrap())
        .build_vk_surface(&events_loop, instance.clone())
        .expect("Couldn't build Vulkan surface");
    info!("Vulkan surface initialized");

    let capabs = surface.capabilities(physical_device)
        .expect("Couldn't acquire surface capabilities");
    info!("Surface capabilities acquired");

    let mut dimensions = capabs.current_extent.unwrap_or([640, 480]);

    let (mut swapchain, mut images) = init_swapchain(device.clone(), surface.clone(), capabs, dimensions)
        .expect("Couldn't initialize swapchain");
    info!("Swapchain initialized");

    info!("Vulkan state finished initialization");

    let queue = queues.nth(0).unwrap();

    let vs = vs::Shader::load(device.clone()).expect("Couldn't create shader module");
    let fs = fs::Shader::load(device.clone()).expect("Couldn't create shader module");
    info!("Shaders initialized");

    let render_pass = init_render_pass(device.clone(), swapchain.clone())
        .expect("Couldn't initialize render pass");
    info!("Render pass initialized");

    let pipeline = Arc::new(vk::pipeline::GraphicsPipeline::start()
        .vertex_input_single_buffer::<Vertex>()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap());
    info!("Pipeline initialized");

    let mut framebuffers: Option<Vec<Arc<Framebuffer<_,_>>>> = None;

    let mut proj = get_projection(dimensions);
    let mut view = Matrix4::look_at(cgmath::Point3::new(0.0, 0.0, 1.0), cgmath::Point3::new(0.0, 0.0, 0.0), cgmath::Vector3::new(0.0, -1.0, 0.0));

    use system::RenderSystem;
    use vk::buffer::BufferUsage;

    let mut local_vertex_buffer = DeviceLocalBuffer::<[Vertex]>::array(
        device.clone(),
        0,
        BufferUsage::all(),
        vec![queue.family()]
    ).expect("Couldn't create local vertex buffer");

    let vertex_buffer = CpuBufferPool::<Vertex>::new(
        device.clone(),
        BufferUsage::vertex_buffer() | BufferUsage::transfer_source(),
    );

    info!("Vertex buffers initialized");

    let mut local_index_buffer = DeviceLocalBuffer::<[u32]>::array(
        device.clone(),
        0,
        BufferUsage::all(),
        vec![queue.family()]
    ).expect("Couldn't create local index buffer");
    
    let index_buffer = CpuBufferPool::<u32>::new(
        device.clone(),
        BufferUsage::index_buffer() | BufferUsage::transfer_source(),
    );

    info!("Index buffers initialized");

    let instance_buffer = CpuBufferPool::<vs::ty::Instance>::new(
        device.clone(),
        BufferUsage::uniform_buffer() | BufferUsage::transfer_source(),
    );
    
    info!("Instance buffer initialized");

    // TODO: Use our custom Game struct to set this up

    let mut world = specs::World::new();

    world.add_resource(res::Device(Some(device.clone())));    
    world.add_resource(res::Queue(Some(queue.clone())));    
    world.add_resource(res::Framebuffer(None));    
    world.add_resource(res::DynamicState(None));

    let (render_sys, cmd_buf_rx) = RenderSystem::new(
        pipeline.clone(), 
        (local_vertex_buffer.clone(), vertex_buffer), 
        (local_index_buffer.clone(), index_buffer), 
        instance_buffer.clone(),
    );

    let mut dispatcher = specs::DispatcherBuilder::new()
        .with(render_sys, "render", &[])
        .build();

    dispatcher.setup(&mut world.res);

    let entity = world.create_entity()
        .with(comp::StaticRender::new(
            vec![
                Vertex { position: [-0.5, -0.5] },
                Vertex { position: [0.5, -0.5] },
                Vertex { position: [-0.5, 0.5] },
                Vertex { position: [0.5, 0.5] },
            ],
            vec![
                0, 1, 2,
                1, 2, 3
            ]
        ))
        .build(); 

    let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;
    let mut recreate_swapchain = false;
    let mut update_vertices = true;
    let mut running = true;
    while running {
        previous_frame_end.cleanup_finished();

        if recreate_swapchain {
            dimensions = surface.capabilities(physical_device)
                .expect("Couldn't acquire surface capabilities")
                .current_extent.unwrap_or([640, 480]);

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                Err(vk::swapchain::SwapchainCreationError::UnsupportedDimensions) => {
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

        let (image_index, acquire_future) = match vk::swapchain::acquire_next_image(swapchain.clone(), None) {
            Ok(r) => r,
            Err(vk::swapchain::AcquireError::OutOfDate) => { 
                recreate_swapchain = true; 
                continue; 
            },
            Err(err) => panic!("{:?}", err)
        };

        (*world.write_resource::<res::Framebuffer>()).0 = Some(
            framebuffers.as_ref().unwrap()[image_index].clone()
        );

        (*world.write_resource::<res::DynamicState>()).0 = Some(
            vk::command_buffer::DynamicState {
                line_width: None,
                viewports: Some(vec![vk::pipeline::viewport::Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0 .. 1.0,
                }]),
                scissors: None,
            }
        );

        dispatcher.dispatch(&world.res);

        let command_buffer = cmd_buf_rx.recv().unwrap();

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

    ortho(w, -w, h, -h, -10., 10.)
}