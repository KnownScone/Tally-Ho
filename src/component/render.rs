use ::Vertex;

use std::sync::Arc;
use std::ops::Range;

use vulkano as vk;
use specs;

// Holds vulkano back-end rendering data
pub struct StaticRender {
    // TODO: Can't this just be a Box<>
    pub instance_set: Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>,

    // TODO: THIS SHIT (individual vbuf/ibuf for each mesh) IS HIGHLY INEFFICIENT; FIX!
    // For now it's easiest way though, so fuck it
    pub vertex_buf: Option<Arc<vk::buffer::ImmutableBuffer<[Vertex]>>>,
    pub vertex_data: Vec<Vertex>,
    
    pub index_buf: Option<Arc<vk::buffer::ImmutableBuffer<[u32]>>>,
    pub index_data: Vec<u32>,
}

impl StaticRender {
    pub fn new(vertex_data: Vec<Vertex>, index_data: Vec<u32>) -> StaticRender {
        StaticRender {
            instance_set: None,
            vertex_buf: None,
            vertex_data,
            index_buf: None,
            index_data,
        }
    }
}

impl specs::Component for StaticRender {
    type Storage = specs::FlaggedStorage<Self, specs::VecStorage<Self>>;
}