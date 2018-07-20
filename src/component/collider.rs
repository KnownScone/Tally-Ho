use ::utility::{Rect2, Rect3};
use ::script::{ScriptResult, ScriptError, ComponentParser};

use std::ops::Range;
use std::sync::Arc;
use std::fmt;

use rlua::{Value as LuaValue, Result as LuaResult, Error as LuaError, Function as LuaFunction, UserData, UserDataMethods, RegistryKey, Table, Lua};
use cgmath::{Zero, Vector2, Vector3};
use specs;

/* NOTE:
    Collision shapes are 2D, but their positions may involve a 3rd axis. Instead of opting to cut off the depth axis
    from the collider position, it will be kept in order to do collision checking. However, the shape itself is a 
    plane: it has no thickness.
*/

#[derive(Debug)]
pub enum Shape {
    AABB(Rect3<f32>),
    Circle {
        /* NOTE:
            The circle's origin should default to (pos.x + radius, pos.y + radius) b/c the 
            transform's position specifies the top-right corner of the entity. 
        */
        // Used to offset the circle's origin.
        offset: Vector2<f32>,
        radius: f32,
        depth: Range<f32>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct Bound {
    pub rect: Rect3<f32>,
}

impl Shape {
    pub fn bound(&self, pos: Vector3<f32>) -> Bound {
        match self {
            &Shape::AABB(r) => Bound {
                rect: Rect3::new(
                    pos + r.min,
                    pos + r.max,
                ),
            },
            &Shape::Circle { offset: o, radius: r, depth: ref d } => Bound {
                rect: Rect3::new(
                    pos + o.extend(d.start),
                    pos + o.extend(d.end) + Vector3::new(r*2.0, r*2.0, 0.0)
                ),
            },
        }
    }
}

pub struct Collider {
    pub shape: Shape,

    pub sweep: bool,

    pub on_collide: Option<RegistryKey>,
    
    // Broad phase index.
    pub index: Option<usize>,
}

impl Collider {
    pub fn new(shape: Shape, sweep: bool, on_collide: Option<RegistryKey>) -> Collider {
        Collider {
            shape,
            sweep,
            on_collide,
            index: None,
        }
    }
}

impl specs::Component for Collider {
    type Storage = specs::storage::BTreeStorage<Self>;
}

impl ComponentParser for Collider { 
    fn parse(v: LuaValue, lua: &Lua) -> ScriptResult<Self> {
        match v {
            LuaValue::Table(t) => {
                let shape_type: String = t.get("shape_type")?;

                let shape = match shape_type.as_ref() {
                    "aabb" => {
                        let t: Table = t.get("shape")?;

                        Shape::AABB(
                            Rect3::new(
                                Vector3::new(
                                    t.get("min_x")?, 
                                    t.get("min_y")?, 
                                    t.get("min_z")?, 
                                ),
                                Vector3::new(
                                    t.get("max_x")?, 
                                    t.get("max_y")?, 
                                    t.get("max_z")?, 
                                )
                            )
                        )
                    },
                    "circle" => {
                        let t: Table = t.get("shape")?;

                        Shape::Circle {
                            offset: {
                                let t: Table = t.get("offset")?;
                                Vector2::new(
                                    t.get("x")?,
                                    t.get("y")?,
                                )
                            },
                            radius: t.get("radius")?,
                            depth:
                                t.get("min_z")?
                                .. t.get("max_z")?
                        }
                    },
                    _ => panic!("Type is not a valid shape")
                };


                let key = {
                    let func: Option<LuaFunction> = t.get("on_collide").ok();
                    func.map(|x| lua.create_registry_value(x).unwrap())
                };

                Ok(Collider::new(
                    shape,
                    t.get("sweep")?,
                    key
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

impl fmt::Debug for Collider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { 
        write!(f, "Collider: shape: {:?}, sweep: {}, index: {:?}", self.shape, self.sweep, self.index) 
    }
}

#[test]
fn get_bound() {
    let circle = Shape::Circle {
        offset: Vector2::new(2.0, 2.0),
        radius: 2.0,
        depth: 0.0..1.0,
    };

    let aabb = Shape::AABB(
        Rect3::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(2.0, 2.0, 1.0)
        )
    );

    let pos = Vector3::new(4.0, 4.0, 0.0);

    assert_eq!(circle.bound(pos), Bound {
        rect: Rect3::new(
            Vector3::new(6.0, 6.0, 0.0),
            Vector3::new(10.0, 10.0, 1.0),
        ),
    });

    assert_eq!(aabb.bound(pos), Bound {
        rect: Rect3::new(
            Vector3::new(4.0, 4.0, 0.0),
            Vector3::new(6.0, 6.0, 1.0),
        ),
    });
}