use ::script::{ScriptResult, ScriptError, ComponentParser};
use ::utility::Rect2;
use ::Vertex;

use std::sync::Arc;

use rlua::{Table, Value as LuaValue, Result as LuaResult, Error as LuaError, UserData, UserDataMethods, Lua};
use cgmath::Vector2;
use vulkano as vk;
use specs;

// Holds vulkano back-end rendering data
pub struct Sprite {
    // TODO: Can't this just be a Box<>
    pub instance_set: Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>,

    pub bounds: Rect2<f32>,
    pub uv: Rect2<f32>,

    pub vertex_buf: Option<Arc<vk::buffer::ImmutableBuffer<[Vertex]>>>,
    pub index_buf: Option<Arc<vk::buffer::ImmutableBuffer<[u32]>>>,

    pub image_index: u32,
}

impl Sprite {
    pub fn new(bounds: Rect2<f32>, uv: Rect2<f32>, image_index: u32) -> Sprite {
        Sprite {
            instance_set: None,
            bounds,
            uv,
            vertex_buf: None,
            index_buf: None,
            image_index,
        }
    }
}

impl specs::Component for Sprite {
    type Storage = specs::FlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl ComponentParser for Sprite { 
    fn parse(v: LuaValue, _: &Lua) -> ScriptResult<Self> {
        match v {
            LuaValue::Table(t) => {
                let bounds = {
                    let t: Table = t.get("bounds")?;
                    Rect2::new(
                        Vector2::new(
                            t.get("min_x")?, 
                            t.get("min_y")?, 
                        ),
                        Vector2::new(
                            t.get("max_x")?, 
                            t.get("max_y")?, 
                        )
                    )
                };

                let uv = {
                    let t: Table = t.get("uv")?;
                    Rect2::new(
                        Vector2::new(
                            t.get("min_x")?, 
                            t.get("min_y")?, 
                        ),
                        Vector2::new(
                            t.get("max_x")?, 
                            t.get("max_y")?, 
                        )
                    )
                };

                Ok(Sprite::new(
                    bounds,
                    uv,
                    t.get("image_index")?
                ))
            },
            LuaValue::Error(err) => Err(ScriptError::LuaError(err)),
            _ => Err(ScriptError::LuaError(LuaError::FromLuaConversionError {
                from: "_",
                to: "table",
                message: None, 
            })),
        }
    }
}