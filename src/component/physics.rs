use ::script::ComponentParser;

use rlua::{Value as LuaValue, Result as LuaResult};
use specs;

#[derive(Debug)]
pub struct Velocity {
    pub x: f32,
    pub y: f32
}

impl specs::Component for Velocity {
    type Storage = specs::VecStorage<Self>;
}

impl ComponentParser for Velocity { 
    fn parse(v: LuaValue) -> LuaResult<Self> {
        match v {
            LuaValue::Table(t) =>
                Ok(Velocity {
                    x: t.get("x").expect("Couldn't get x-vel"),
                    y: t.get("y").expect("Couldn't get y-vel")
                }),
            LuaValue::Error(err) => Err(err),
            _ => panic!("Couldn't parse the data to a component, not of the proper type"),
        }
    }
}