use ::script::ComponentParser;

use rlua::{Value as LuaValue, Result as LuaResult, Error as LuaError};
use cgmath::Vector3;
use specs;

#[derive(Debug)]
pub struct Velocity {
    pub pos: Vector3<f32>
}

impl specs::Component for Velocity {
    type Storage = specs::VecStorage<Self>;
}

impl ComponentParser for Velocity { 
    fn parse(v: LuaValue) -> LuaResult<Self> {
        match v {
            LuaValue::Table(t) =>
                Ok(Velocity {
                    pos: Vector3::new(
                        t.get("x").expect("Couldn't get x-pos"), 
                        t.get("y").expect("Couldn't get y-pos"), 
                        t.get("z").expect("Couldn't get z-pos")
                    ),
                }),
            LuaValue::Error(err) => Err(err),
            _ => Err(LuaError::FromLuaConversionError {
                from: "_",
                to: "table",
                message: None, 
            }),
        }
    }
}