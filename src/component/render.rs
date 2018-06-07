use ::script::ComponentParser;
use ::Vertex;

use std::sync::Arc;

use rlua::{Value as LuaValue, Result as LuaResult, Error as LuaError};
use vulkano as vk;
use specs;

// Holds vulkano back-end rendering data
pub struct Render {
    // TODO: Can't this just be a Box<>
    pub instance_set: Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>,

    // TODO: THIS SHIT (individual vbuf/ibuf for each mesh) IS HIGHLY INEFFICIENT; FIX!
    // For now it's easiest way though, so fuck it
    pub vertex_buf: Option<Arc<vk::buffer::ImmutableBuffer<[Vertex]>>>,
    pub vertex_data: Vec<Vertex>,
    
    pub index_buf: Option<Arc<vk::buffer::ImmutableBuffer<[u32]>>>,
    pub index_data: Vec<u32>,

    pub image_index: u32,
}

impl Render {
    pub fn new(vertex_data: Vec<Vertex>, index_data: Vec<u32>, image_index: u32) -> Render {
        Render {
            instance_set: None,
            image_index,
            vertex_buf: None,
            vertex_data,
            index_buf: None,
            index_data,
        }
    }
}

impl specs::Component for Render {
    type Storage = specs::FlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl ComponentParser for Render { 
    fn parse(v: LuaValue) -> LuaResult<Self> {
        match v {
            LuaValue::Table(t) => {
                let shape: String = t.get("shape").expect("Couldn't get shape");
                let (verts, idxs) = match shape.as_ref() {
                    "Quad" => (
                        vec![
                            Vertex { position: [-0.5, -0.5], uv: [0.0, 0.0], },
                            Vertex { position: [0.5, -0.5], uv: [1.0, 0.0] },
                            Vertex { position: [-0.5, 0.5], uv: [0.0, 1.0] },
                            Vertex { position: [0.5, 0.5], uv: [1.0, 1.0] },
                        ],
                        vec![
                            0, 1, 2,
                            1, 2, 3
                        ]),
                    _ => panic!("Not a valid shape")
                };
                Ok(Render::new(
                    verts,
                    idxs,
                    t.get("image_index").expect("Couldn't get image index")
                ))
            },
            LuaValue::Error(err) => Err(err),
            _ => Err(LuaError::FromLuaConversionError {
                from: "_",
                to: "table",
                message: None, 
            }),
        }
    }
}