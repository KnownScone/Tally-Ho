mod render;
pub use self::render::{RenderId, SortedRender};

pub mod input;
pub use self::input::{InputList};

use script;

use std::sync::{Mutex, Arc};

use vulkano as vk;

#[derive(Default)]
pub struct DeltaTime(pub f32);

#[derive(Default)]
pub struct Script(pub Option<Arc<Mutex<script::Script>>>);

#[derive(Default)]
pub struct ViewProjectionSet(pub Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>);

#[derive(Default)]
pub struct TextureSet(pub Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>);

#[derive(Default)]
pub struct Device(pub Option<Arc<vk::device::Device>>);

#[derive(Default)]
pub struct Queue(pub Option<Arc<vk::device::Queue>>);

#[derive(Default)]
pub struct Framebuffer(pub Option<Arc<vk::framebuffer::FramebufferAbstract + Send + Sync>>);

#[derive(Default)]
pub struct DynamicState(pub Option<vk::command_buffer::DynamicState>);