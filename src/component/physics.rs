use ::script::{ScriptResult, ScriptError, ComponentParser};

use rlua::{Value as LuaValue, Result as LuaResult, Error as LuaError, UserData, UserDataMethods, Lua};
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
    fn parse(v: LuaValue, _: &Lua) -> ScriptResult<Self> {
        match v {
            LuaValue::Table(t) =>
                Ok(Velocity {
                    pos: Vector3::new(
                        t.get("x")?, 
                        t.get("y")?, 
                        t.get("z")?
                    ),
                }),
            LuaValue::Error(err) => Err(ScriptError::LuaError(err)),
            _ => Err(ScriptError::LuaError(LuaError::FromLuaConversionError {
                from: "_",
                to: "table",
                message: None, 
            })),
        }
    }
}