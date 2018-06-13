use ::Vertex;
use ::utility::Rect2;

use std::sync::Arc;

use cgmath::{Point2, Point3, Vector2, Vector3, Transform};
use vulkano as vk;
use specs;

const STRIP_LENGTH: u32 = 10;

pub struct TileMap {
    pub instance_set: Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>,
    pub tile_dims: Vector3<f32>,

    pub image_index: u32,

    pub strips: Vec<Strip>,
}

impl TileMap {
    pub fn new(tile_dims: Vector3<f32>, image_index: u32) -> TileMap {
        TileMap {
            instance_set: None,
            tile_dims,
            image_index,
            strips: Vec::new()
        }
    }

    pub fn create_strip(&mut self, queue: Arc<vk::device::Queue>, strip_pos: Point3<u32>, tile_uvs: [Rect2<f32>; STRIP_LENGTH as usize]) {
        let world_pos = Vector3::new(
            (strip_pos.x * STRIP_LENGTH) as f32 * self.tile_dims.x,
            strip_pos.y as f32 * self.tile_dims.y,
            strip_pos.z as f32 * self.tile_dims.z
        );
        
        let vertex_data: Vec<_> = tile_uvs.iter().enumerate()
            .flat_map(|(idx, uv)| {
                let local_pos = Vector3::new(
                    idx as f32 * self.tile_dims.x,
                    0.0,
                    0.0
                );

                vec![
                    Vertex {
                        position: (world_pos + local_pos).into(),
                        uv: [uv.min.x, uv.min.y]
                    },
                    Vertex {
                        position: (world_pos + local_pos + Vector3::new(self.tile_dims.x, 0.0, 0.0)).into(),
                        uv: [uv.max.x, uv.min.y]
                    },
                    Vertex {
                        position: (world_pos + local_pos + Vector3::new(0.0, self.tile_dims.y, 0.0)).into(),
                        uv: [uv.min.x, uv.max.y]
                    },
                    Vertex {
                        position: (world_pos + local_pos + Vector3::new(self.tile_dims.x, self.tile_dims.y, 0.0)).into(),
                        uv: [uv.max.x, uv.max.y]
                    }
                ]
            })
        .collect();

        let index_data: Vec<_> = tile_uvs.iter().enumerate()
            .flat_map(|(idx, _)| {
                let i = idx as u32 * 4;
                vec![
                    i, i + 1, i + 2,
                    i + 1, i + 2, i + 3
                ]
            })
        .collect();

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

        self.strips.push(Strip {
            pos: strip_pos,
            vertex_buf,
            index_buf
        });
    }
}

pub struct Strip {
    //pub tiles: [u32; (CHUNK_SIZE.x * CHUNK_SIZE.y) as usize],
    pub pos: Point3<u32>,
    
    pub vertex_buf: Arc<vk::buffer::ImmutableBuffer<[Vertex]>>,
    pub index_buf: Arc<vk::buffer::ImmutableBuffer<[u32]>>,
}

impl specs::Component for TileMap {
    type Storage = specs::FlaggedStorage<Self, specs::storage::BTreeStorage<Self>>;
}