use ::utility::{Rect2, Rect3};

use cgmath::{Zero, Vector2, Vector3};
use specs;

/* NOTE:
    Collision shapes are 2D, but their positions may involve a 3rd axis. Instead of opting to cut off the depth axis
    from the collider position, it will be kept in order to do collision checking. However, the shape itself is a 
    plane: it has no thickness.
*/

#[derive(Debug)]
pub enum Shape {
    AABB(Rect2<f32>),
    Circle {
        /* NOTE:
            The circle's origin should default to (pos.x + radius, pos.y + radius) b/c the 
            transform's position specifies the top-right corner of the entity. 
        */
        // Used to offset the circle's origin.
        offset: Vector2<f32>,
        radius: f32,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct Bound {
    pub rect: Rect2<f32>,
    pub depth: f32,
}

impl Shape {
    pub fn bound(&self, pos: Vector3<f32>) -> Bound {
        match self {
            &Shape::AABB(r) => Bound {
                rect: Rect2::new(
                    pos.truncate() + r.min,
                    pos.truncate() + r.max,
                ),
                depth: pos.z
            },
            &Shape::Circle { offset: o, radius: r } => {
                let d = Vector2::new(r*2.0, r*2.0);
                Bound {
                    rect: Rect2::new(
                        pos.truncate() + o,
                        pos.truncate() + o + d
                    ),
                    depth: pos.z
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct Collider {
    pub shape: Shape,
    pub bound: Option<Bound>,

    pub sweep: bool,
    pub last_pos: Vector3<f32>
}

impl Collider {
    pub fn new(shape: Shape) -> Collider {
        Collider {
            shape,
            bound: None,
            sweep: true,
            last_pos: Vector3::zero(),
        }
    }
}

impl specs::Component for Collider {
    type Storage = specs::storage::BTreeStorage<Self>;
}

#[test]
fn get_bound() {
    let circle = Shape::Circle {
        offset: Vector2::new(2.0, 2.0),
        radius: 2.0,
    };

    let aabb = Shape::AABB(
        Rect2::new(
            Vector2::new(0.0, 0.0),
            Vector2::new(2.0, 2.0)
        )
    );

    let pos = Vector3::new(4.0, 4.0, 0.0);

    assert_eq!(circle.bound(pos), Bound {
        rect: Rect2::new(
            Vector2::new(6.0, 6.0),
            Vector2::new(10.0, 10.0),
        ),
        depth: 0.0
    });

    assert_eq!(aabb.bound(pos), Bound {
        rect: Rect2::new(
            Vector2::new(4.0, 4.0),
            Vector2::new(6.0, 6.0),
        ),
        depth: 0.0
    });
}