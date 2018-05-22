use ::script::ComponentParser;

use rlua::{Value as LuaValue, Result as LuaResult, Error as LuaError};
use specs;

#[derive(Debug)]
pub struct Transform {
    pub x: f32,
    pub y: f32
}

impl specs::Component for Transform {
    type Storage = specs::VecStorage<Self>;
}

impl ComponentParser for Transform { 
    fn parse(v: LuaValue) -> LuaResult<Self> {
        match v {
            LuaValue::Table(t) =>
                Ok(Transform {
                    x: t.get("x").expect("Couldn't get x-pos"),
                    y: t.get("y").expect("Couldn't get y-pos")
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