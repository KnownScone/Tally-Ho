use ::resource::Mesh;
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

    pub mesh_index: usize,

    pub image_index: u32,
}

impl Render {
    pub fn new(mesh_index: usize, image_index: u32) -> Render {
        Render {
            instance_set: None,
            mesh_index,
            image_index,
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
                Ok(Render::new(
                    t.get("mesh_index").expect("Couldn't get mesh"),
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