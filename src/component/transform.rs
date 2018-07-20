use ::script::{ScriptResult, ScriptError, ComponentParser};

use rlua::{Table, Value as LuaValue, Result as LuaResult, Error as LuaError, UserData, UserDataMethods, Lua};
use cgmath::{Vector3};
use specs;

#[derive(Debug)]
pub struct Transform {
    pub last_pos: Vector3<f32>,
    pub pos: Vector3<f32>,
}

impl Transform {
    pub fn new(pos: Vector3<f32>) -> Self {
        Transform {
            pos,
            last_pos: pos,
        }
    }
}

impl specs::Component for Transform {
    type Storage = specs::FlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl ComponentParser for Transform { 
    fn parse(v: LuaValue, _: &Lua) -> ScriptResult<Self> {
        match v {
            LuaValue::Table(t) => {
                let pos = {
                    let t: Table = t.get("position")?;
                    Vector3::new(
                        t.get("x")?, 
                        t.get("y")?, 
                        t.get("z")?
                    )
                };

                Ok(Transform::new(pos))
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