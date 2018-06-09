use ::Vertex;
use ::utility::Rect2;

use std::sync::Arc;

use cgmath::{Point2, Point3, Vector2, Vector3, Transform};
use vulkano as vk;
use specs;

const CHUNK_SIZE: Vector2<u32> = Vector2 { x: 5, y: 5 };

pub struct TileMap {
    pub instance_set: Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>,
    pub tile_dims: Vector3<f32>,

    pub image_index: u32,

    pub chunks: Vec<Chunk>,
}

impl TileMap {
    pub fn new(tile_dims: Vector3<f32>, image_index: u32) -> TileMap {
        TileMap {
            instance_set: None,
            tile_dims,
            image_index,
            chunks: Vec::new()
        }
    }

    pub fn create_chunk(&mut self, queue: Arc<vk::device::Queue>, chunk_pos: Point3<u32>, tile_uvs: [Rect2<f32>; (CHUNK_SIZE.x * CHUNK_SIZE.y) as usize]) {
        let chunk_pos = Vector3::new(
            (chunk_pos.x * CHUNK_SIZE.x) as f32 * self.tile_dims.x,
            (chunk_pos.y * CHUNK_SIZE.y) as f32 * self.tile_dims.y,
            chunk_pos.z as f32 * self.tile_dims.z
        );
        
        let vertex_data: Vec<_> = tile_uvs.iter().enumerate()
            .flat_map(|(idx, uv)| {
                let local_pos = Vector2::new(
                    (idx as f32 % CHUNK_SIZE.x as f32) * self.tile_dims.x,
                    (idx as f32 / CHUNK_SIZE.y as f32).floor() * self.tile_dims.y,
                );

                vec![
                    Vertex {
                        position: (chunk_pos + local_pos.extend(0.0)).into(),
                        uv: [uv.min.x, uv.min.y]
                    },
                    Vertex {
                        position: (chunk_pos + local_pos.extend(0.0) + Vector3::new(self.tile_dims.x, 0.0, 0.0)).into(),
                        uv: [uv.max.x, uv.min.y]
                    },
                    Vertex {
                        position: (chunk_pos + local_pos.extend(0.0) + Vector3::new(0.0, self.tile_dims.y, 0.0)).into(),
                        uv: [uv.min.x, uv.max.y]
                    },
                    Vertex {
                        position: (chunk_pos + local_pos.extend(0.0) + Vector3::new(self.tile_dims.x, self.tile_dims.y, 0.0)).into(),
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

        self.chunks.push(Chunk {
            vertex_buf,
            index_buf
        });
    }
}

pub struct Chunk {
    //pub tiles: [u32; (CHUNK_SIZE.x * CHUNK_SIZE.y) as usize],
    pub vertex_buf: Arc<vk::buffer::ImmutableBuffer<[Vertex]>>,
    pub index_buf: Arc<vk::buffer::ImmutableBuffer<[u32]>>,
}

impl specs::Component for TileMap {
    type Storage = specs::FlaggedStorage<Self, specs::storage::BTreeStorage<Self>>;
}