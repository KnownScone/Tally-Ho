use utility::Rect2;
use ::script::ComponentParser;

use rlua::{Table, Value as LuaValue, Result as LuaResult, Error as LuaError};
use cgmath::{Vector2, Vector3};
use specs;

#[derive(Debug)]
pub struct Transform {
    pub pos: Vector3<f32>,
    pub bounds: Rect2<f32>
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
                        t.get("x").expect("Couldn't get x-pos"), 
                        t.get("y").expect("Couldn't get y-pos"), 
                        t.get("z").expect("Couldn't get z-pos")
                    )
                };

                let bounds = {
                    let t: Table = t.get("bounds").expect("Couldn't get bounds");
                    Rect2::new(
                        Vector2::new(
                            t.get("min_x").expect("Couldn't get min_x"), 
                            t.get("max_x").expect("Couldn't get max_x"), 
                        ),
                        Vector2::new(
                            t.get("min_y").expect("Couldn't get min_y"), 
                            t.get("max_y").expect("Couldn't get max_y"), 
                        )
                    )
                };

                Ok(Transform {
                    pos,
                    bounds,
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