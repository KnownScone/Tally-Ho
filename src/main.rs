extern crate chrono;
extern crate fern;
#[macro_use]
extern crate log;
#[macro_use]
extern crate vulkano;
extern crate vulkano_win;
extern crate winit;

mod game;
mod scene;

use std::sync::Arc;
use std::cmp::{max, min};

use vulkano::swapchain::{Surface, Swapchain, CompositeAlpha, PresentMode};
use vulkano::device::{Device};
use vulkano::image::{ImageUsage};
use vulkano::sync::{SharingMode};
use vulkano::instance::{Features, Instance, InstanceExtensions, DeviceExtensions, PhysicalDevice};

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
            &features, 
            &extensions, 
            queue_family.map(|queue| (queue, 1.0))
        ).expect("Couldn't build device")
    };

    let events_loop = EventsLoop::new();

    let surface = WindowBuilder::new()
        .with_title("Tally Ho")
        .build_vk_surface(&events_loop, instance.clone())
        .expect("Couldn't build Vulkan surface");
    
    let capabs = surface.capabilities(device.physical_device())
        .expect("Couldn't acquire surface capabilities");

    let dimensions = capabs.current_extent.unwrap_or([640, 480]);

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

    let (swapchain, buffers) = Swapchain::new(
        device,
        surface,
        buffers_count,
        format,
        dimensions,
        1,
        usage,
        sharing_mode,
        transform,
        alpha,
        PresentMode::Fifo,
        true,
        None
    ).expect("Couldn't build swapchain");

    info!("Window initialized");
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