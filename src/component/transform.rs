use ::script::ComponentParser;

use rlua::{Table, Value as LuaValue, Result as LuaResult, Error as LuaError};
use cgmath::{Vector3};
use specs;

#[derive(Debug)]
pub struct Transform {
    pub pos: Vector3<f32>,
}

impl specs::Component for Transform {
    type Storage = specs::FlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl ComponentParser for Transform { 
    fn parse(v: LuaValue) -> LuaResult<Self> {
        match v {
            LuaValue::Table(t) => {
                let pos = {
                    let t: Table = t.get("position").expect("Couldn't get position");
                    Vector3::new(
                        t.get("x").expect("Couldn't get x"), 
                        t.get("y").expect("Couldn't get y"), 
                        t.get("z").expect("Couldn't get z")
                    )
                };

                Ok(Transform {
                    pos,
                })
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