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
extern crate dmsort;
#[macro_use]
extern crate cgmath;
extern crate image;
extern crate specs;
extern crate rlua;
#[macro_use]
extern crate nom;
extern crate num_traits;
#[macro_use]
extern crate num_derive;

mod collision;
mod resource;
mod component;
mod utility;
mod system;
mod script;
mod parse;
mod game;

use resource as res;
use component as comp;
use system as sys;

use std::sync::Arc;
use std::cmp::{max, min};

use vulkano as vk;
use vk::instance::{Instance, PhysicalDevice};
use vk::swapchain::{Swapchain};
use vk::framebuffer::{Framebuffer};
use vk::buffer::{CpuBufferPool, DeviceLocalBuffer, BufferUsage};
use vk::command_buffer::{AutoCommandBufferBuilder};
use vk::descriptor::descriptor_set::FixedSizeDescriptorSetsPool;
use vk::sync::{now, GpuFuture};
use vk::device::{Device};

use cgmath::{ortho, Matrix4};

use winit::{EventsLoop, WindowBuilder};

use vulkano_win::VkSurfaceBuild;

mod vs {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[src = "
#version 450
layout(location = 0) in vec3 position;
layout(location = 1) in vec2 uv;

layout(set = 0, binding = 0) uniform Instance {
    mat4 transform;
} instance;

layout(set = 1, binding = 0) uniform ViewProjection {
    mat4 view;
    mat4 proj;
} viewProj;

layout(location = 0) out vec2 f_uv;

void main() {
    gl_Position = viewProj.proj * viewProj.view * instance.transform * vec4(position, 1.0);
    f_uv = uv;
}
"]
    #[allow(dead_code)]
    struct Dummy;
}

mod fs {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[src = "
#version 450
layout(set = 2, binding = 0) uniform sampler samp;
layout(set = 2, binding = 1) uniform texture2D textures[4];

layout(push_constant) uniform PER_OBJECT
{
    uint imgIdx;
} pc;

layout(location = 0) out vec4 f_color;
layout(location = 0) in vec2 f_uv;

void main() {
    f_color = texture(sampler2D(textures[pc.imgIdx], samp), f_uv);
}
"]
    #[allow(dead_code)]
    struct Dummy;
}

#[derive(Debug, Clone)]
pub struct Vertex { 
    position: [f32; 3],
    uv: [f32; 2],
}

impl_vertex!(Vertex, position, uv);

pub fn select_physical_device<'a>(instance: &'a Arc<Instance>) -> Option<PhysicalDevice<'a>> {
    // TODO: Better physical device selection.
    PhysicalDevice::from_index(
        instance, 
        0
    )
}

fn main() {
    // TODO: Handle this better, rather than just a panic.
    init_logging().unwrap();

    info!("Logging initialized");

    // Holds the application's name and version. Built from 'Cargo.toml' at compile-time.
    let app_info = app_info_from_cargo_toml!();

    // Instance of a Vulkan context, essential to the rest of the application.
    let instance = {
        // List of extensions that must be enabled on the newly-created instance.
        // * It is not possible to use the features of an extension if it was not explicitly enabled.
        let extensions = vulkano_win::required_extensions();

        Instance::new(
            Some(&app_info), 
            &extensions, 
            // ? Use the instance 'layers' for debugging?
            None
        ).expect("Couldn't initialize instance")
    };
    info!("Instance initialized");

    // Selects a physical device from the ones available on the system.
    let physical_device = select_physical_device(&instance)
        .expect("Couldn't select physical device");
    info!("Physical device selected");

    // The device and an iterator over the created queues.
    // A queue is a CPU thread, executing it's commands one after another, that is used to submit commands to the GPU. 
    let (device, mut queues) = {
        // Device extensions are similar to instance extensions, except they are for the device. 
        // * It is not possible to use the functions of a extension if it was not explicitly enabled.
        let extensions = vk::instance::DeviceExtensions {
            khr_swapchain: true,
            .. vk::instance::DeviceExtensions::none()
        };

        // Features are similar too, except they are part of the core Vulkan specs instead of being separate documents.
        // * It is not possible to use the functions of a feature if it was not explicitly enabled.
        let features = physical_device.supported_features();

        // TODO: Better handling of queues (specifying priorities, etc.)
        // List of queues to create, each element indicates it's family and priority (0.0 - 1.0).
        // Queues are divided in queue families, all the queues within have the same characteristics.
        // * No guarantee can be made on the way the priority is handled by the implementation.        
        let queues = physical_device.queue_families()
            .map(|family| 
                (family, 1.0)
            );

        Device::new(
            physical_device, 
            &features, 
            &extensions,
            queues,
        ).expect("Couldn't initialize device")
    };

    info!("Device initialized");
    info!("Queues initialized");

    // Provides a way to retrieve events from the system and from the windows that were registered.
    let mut events_loop = EventsLoop::new();
    info!("Events loop initialized");

    // Builds a Vulkan surface on the screen using winit.
    let surface = WindowBuilder::new()
        .with_title(app_info.application_name.unwrap())
        .build_vk_surface(&events_loop, instance.clone())
        .expect("Couldn't build surface");
    info!("Surface initialized");

    // Acquires the capabilities of the Vulkan surface when used by the physical device.
    let capabs = surface.capabilities(physical_device)
        .expect("Couldn't acquire surface capabilities");
    info!("Surface capabilities acquired");

    // Keeps track of the proper dimensions, allowing modification throughout the runtime.
    let mut dimensions = capabs.current_extent.unwrap_or([640, 480]);

    // The swapping system and the images that can be shown on the Vulkan surface.
    // * The order in which the images are returned is important for the acquire_next_image and present functions.
    let (mut swapchain, mut images) = {
        // Try to use double-buffering.
        let buffers_count = max(min(2, capabs.min_image_count), capabs.max_image_count.unwrap_or(2));

        let transform = capabs.current_transform;

        let (format, _color_space) = capabs.supported_formats[0];

        let usage = vk::image::ImageUsage {
            .. capabs.supported_usage_flags
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
            usage,
            sharing_mode,
            transform,
            alpha,
            present_mode,
            true,
            None
        ).expect("Couldn't initialize swapchain")
    };
    info!("Swapchain initialized");

    let queue = queues.nth(0).unwrap();

    let vs = vs::Shader::load(device.clone()).expect("Couldn't create shader module");
    let fs = fs::Shader::load(device.clone()).expect("Couldn't create shader module");
    info!("Shaders initialized");

    // Defines layout of the subpass(es).
    let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
        attachments: {
            // Custom name we give to the first and only attachment.
            color: {
                // GPU should clear the content of this attachment at the start of the drawing.
                load: Clear,
                // GPU should store the output of the draw in the actual image. We could also ask it to discard the result.
                store: Store,
                // Indicates the type of the format of the image. Here we use the format specified by the swapchain.
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            // We use the attachment named color as the one and only color attachment.
            color: [color],
            // No depth-stencil attachment is indicated with empty brackets.
            depth_stencil: {}
        }
    ).expect("Couldn't initialize render pass"));

    info!("Render pass initialized");

    // Defines how to perform a draw operation.
    let pipeline = Arc::new(vk::pipeline::GraphicsPipeline::start()
        .vertex_input_single_buffer::<Vertex>()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .render_pass(vk::framebuffer::Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap());
    info!("Pipeline initialized");

    // List of frame-buffers that each contain a render pass and the image views attached to it.
    let mut framebuffers: Option<Vec<Arc<Framebuffer<_,_>>>> = None;

    let mut proj = get_projection(dimensions);
    let mut view = Matrix4::look_at_dir(cgmath::Point3::new(0.0, 0.0, -1.0), cgmath::Vector3::new(0.0, 0.0, 1.0), cgmath::Vector3::new(0.0, 1.0, 0.0));

    let view_proj_buffer = CpuBufferPool::<vs::ty::ViewProjection>::new(
        device.clone(),
        BufferUsage::uniform_buffer() | BufferUsage::transfer_source(),
    );

    let local_view_proj_buffer = DeviceLocalBuffer::<vs::ty::ViewProjection>::new(
        device.clone(),
        BufferUsage::uniform_buffer_transfer_destination(),
        vec![queue.family()]
    ).expect("Couldn't create uniform device local buffer");

    let mut view_proj_set = Arc::new(vulkano::descriptor::descriptor_set::PersistentDescriptorSet::start(pipeline.clone(), 1)
        .add_buffer(local_view_proj_buffer.clone()).unwrap()
        .build().unwrap()
    );

    let (texture, tex_future) = {
        let loaded = image::open("assets/images/ultra_thunk.png").unwrap().to_rgba();
        let dims = loaded.dimensions();
        let image_data = loaded.into_raw().clone();

        vk::image::ImmutableImage::from_iter(
            image_data.iter().cloned(),
            vk::image::Dimensions::Dim2d {
                width: dims.0,
                height: dims.1
            },
            vk::format::R8G8B8A8Srgb,
            queue.clone()
        ).unwrap()
    };
    info!("Immutable texture image created");
    
    let sampler = vk::sampler::Sampler::simple_repeat_linear(device.clone());

    info!("Sampler created");

    let tex_set = Arc::new(vulkano::descriptor::descriptor_set::PersistentDescriptorSet::start(pipeline.clone(), 2)
        .add_sampler(sampler).unwrap()
        .enter_array().unwrap()
            .add_image(texture.clone()).unwrap()
            .add_image(texture.clone()).unwrap()
            .add_image(texture.clone()).unwrap()
            .add_image(texture.clone()).unwrap()
        .leave_array().unwrap()
        .build().unwrap());
    info!("Texture set initialized");

    let tile_map_sys = {
        let instance_sets = FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0);
        let instance_buf = CpuBufferPool::<vs::ty::Instance>::new(
            device.clone(),
            vk::buffer::BufferUsage::uniform_buffer() | vk::buffer::BufferUsage::transfer_source(),
        );

        sys::TileMapSystem::new(instance_sets, instance_buf)
    };
    
    let sprite_sys = {
        let instance_sets = FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0);
        let instance_buf = CpuBufferPool::<vs::ty::Instance>::new(
            device.clone(),
            vk::buffer::BufferUsage::uniform_buffer() | vk::buffer::BufferUsage::transfer_source(),
        );
    
        sys::SpriteSystem::new(instance_sets, instance_buf)
    };

    let (render_sys, cmd_buf_rx) = sys::RenderSystem::new(pipeline.clone());

    let velocity_sys = sys::VelocitySystem;
    
    let collision_sys = sys::CollisionSystem::new();

    let mut logic_disp = specs::DispatcherBuilder::new()
        .with(velocity_sys, "velocity", &[])
        .with(collision_sys, "collision", &["velocity"])
        .build();

    let mut render_disp = specs::DispatcherBuilder::new()
        .with(sprite_sys, "sprite", &[])
        .with(tile_map_sys, "tile_map", &[])
        .with(render_sys, "render", &["sprite", "tile_map"])
        .build();

    let mut game = game::Game::new(1.0/60.0, logic_disp, render_disp);

    let mut script = script::Script::new();
    script.register::<comp::Transform>("transform");
    script.register::<comp::Sprite>("sprite");
    script.register::<comp::Velocity>("velocity");
    script.register::<comp::TileMap>("tile_map");
    script.register::<comp::Collider>("collider");
    script.load_file("assets/scripts/test.lua");

    let _e = script.parse_entity("stuff", game.world.create_entity()).unwrap();
    let _e = script.parse_entity("stuff2", game.world.create_entity()).unwrap();
    // let e = script.parse_entity("stuff_map", game.world.create_entity()).unwrap();
    
    let mut tile_map = comp::TileMap::new(
        cgmath::Vector3::new(0.1, 0.1, 0.1),
        cgmath::Vector2::new(2, 2),
        0,
    );
    
    let parsed_tile_map = parse::tile_map(b"\x05\x04dust\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x02\x01\x00\
    \x00\x00\x00\x01\x00\x00\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01\
    \x00\x02\x00\x03\x00\x02\x00\x03\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01").unwrap().1;
    tile_map.load(parsed_tile_map);

    // let _e = game.world.create_entity()
    //     .with(
    //         tile_map
    //     )
    //     .with(comp::Transform {
    //         pos: cgmath::Vector3::new(0.0, 0.0, 0.0),
    //     })
    // .build(); 

    game.world.add_resource(res::TextureSet(Some(tex_set)));   
    game.world.add_resource(res::ViewProjectionSet(Some(view_proj_set.clone())));   
    game.world.add_resource(res::Device(Some(device.clone())));   
    game.world.add_resource(res::Queue(Some(queue.clone())));    
    game.world.add_resource(res::Framebuffer(None));    
    game.world.add_resource(res::DynamicState(None));

    // Accumulates previous frames' futures until the GPU is done executing them.
    // * Submitting a command produces a future, which holds required resources for as long as they are in use by the GPU.
    let mut previous_frame_end = Box::new(tex_future) as Box<GpuFuture>;
    let mut recreate_swapchain = false;
    let mut running = true;
    while running {
        // Frees up resources that are no longer needed by checking what the GPU has already processed.
        previous_frame_end.cleanup_finished();

        if recreate_swapchain {
            dimensions = surface.capabilities(physical_device)
                .expect("Couldn't acquire surface capabilities")
                .current_extent.unwrap_or([640, 480]);

            // Recreate the swapchain and its images with the new dimensions.
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
            
            // With new swapchain images, we recreate the frame buffers.
            framebuffers = None;

            recreate_swapchain = false;
        }

        if framebuffers.is_none() {
            // Builds each frame buffer with the render pass and their corresponding image views.
            let new_framebuffers = Some(images.iter().map(|image| {
                Arc::new(Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap())
            }).collect::<Vec<_>>());
            std::mem::replace(&mut framebuffers, new_framebuffers);
        }

        // Blocks until able to acquire a drawable image from the swapchain. Returns the index of that image.
        let (image_index, acquire_future) = match vk::swapchain::acquire_next_image(swapchain.clone(), None) {
            Ok(r) => r,
            Err(vk::swapchain::AcquireError::OutOfDate) => { 
                recreate_swapchain = true; 
                continue; 
            },
            Err(err) => panic!("{:?}", err)
        };

        // Passes this frame's available frame buffer into a resource.
        (*game.world.write_resource::<res::Framebuffer>()).0 = Some(
            framebuffers.as_ref().unwrap()[image_index].clone()
        );

        // TODO: Create the DynamicState somewhere else, it only needs an update when the dimensions change.
        (*game.world.write_resource::<res::DynamicState>()).0 = Some(
            vk::command_buffer::DynamicState {
                line_width: None,
                // List of viewports, the region of the image corresponding to the vertex coords -1.0 to 1.0.
                viewports: Some(vec![vk::pipeline::viewport::Viewport {
                    // Coordinates of the top-left corner of the viewport (in pixels).
                    origin: [0.0, 0.0],
                    // Dimensions of the viewport (in pixels).
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    // The range to map z-coords with before comparison with other depth values.                    
                    depth_range: 0.0 .. 1.0,
                }]),
                // List of scissor boxes, any pixel outside of the scissor box is discarded.
                scissors: None,
            }
        );

        game.update(1.0);

        // Receives the render system's command buffer for execution.
        let render_command_buffer = cmd_buf_rx.recv().unwrap();
        
        let view_proj_data = vs::ty::ViewProjection {
            view: view.into(),
            proj: proj.into(),
        };

        let misc_command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
            .copy_buffer(
                view_proj_buffer.next(view_proj_data).unwrap(),
                local_view_proj_buffer.clone(),
            ).unwrap()
            .build().unwrap();

        // Joins previous frames' accumulated futures with the new future.
        let future = previous_frame_end.join(acquire_future)
            // Submits a command to execute our command buffer on the selected queue.
            .then_execute(queue.clone(), misc_command_buffer).unwrap()
            .then_execute(queue.clone(), render_command_buffer).unwrap()
            // Submits a command to present the image at the end of the queue (after executing previous commands).
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

    ortho(w, -w, -h, h, -10., 10.)
}