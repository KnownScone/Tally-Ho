use ::Vertex;

use std::sync::Arc;

use vulkano as vk;

pub struct Mesh {
    pub vertex_buf: Arc<vk::buffer::ImmutableBuffer<[Vertex]>>,
    pub index_buf: Arc<vk::buffer::ImmutableBuffer<[u32]>>,
}

impl Mesh {
    pub fn new(vertex_data: Vec<Vertex>, index_data: Vec<u32>, queue: Arc<vk::device::Queue>) -> Mesh {
        let (vertex_buf, _) = vk::buffer::ImmutableBuffer::from_iter(
            vertex_data.iter().cloned(),
            vk::buffer::BufferUsage::vertex_buffer(),
            queue.clone()
        ).expect("Couldn't create vertex buffer");

        let (index_buf, _) = vk::buffer::ImmutableBuffer::from_iter(
            index_data.iter().cloned(),
            vk::buffer::BufferUsage::index_buffer(),
            queue.clone()
        ).expect("Couldn't create index buffer");

        Mesh {
            vertex_buf,
            index_buf,
        }
    }
}

#[derive(Default)]
pub struct MeshList(pub Vec<Mesh>);