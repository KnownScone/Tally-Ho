use ::script::{ScriptResult, ScriptError, ComponentParser};

use rlua::{Table, Value as LuaValue, Result as LuaResult, Error as LuaError, Function as LuaFunction, UserData, UserDataMethods, Lua, RegistryKey};
use cgmath::{Vector3};
use specs;

pub struct ScriptBehavior {
    pub on_tick: Option<RegistryKey>,
}

impl ScriptBehavior {
    pub fn new(on_tick: Option<RegistryKey>) -> Self {
        ScriptBehavior {
            on_tick,
        }
    }
}

impl specs::Component for ScriptBehavior {
    type Storage = specs::FlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl ComponentParser for ScriptBehavior { 
    fn parse(v: LuaValue, lua: &Lua) -> ScriptResult<Self> {
        match v {
            LuaValue::Table(t) => {
                let key = {
                    let func: Option<LuaFunction> = t.get("on_tick").ok();
                    func.map(|x| lua.create_registry_value(x).unwrap())
                };

                Ok(ScriptBehavior::new(key))
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