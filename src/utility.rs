use cgmath::{Vector2};

#[derive(Debug, Clone, Copy)]
pub struct Rect2<S> {
    pub min: Vector2<S>, 
    pub max: Vector2<S>, 
}

impl<S> Rect2<S> {
    pub fn new(min: Vector2<S>, max: Vector2<S>) -> Self {
        Rect2 {
            min,
            max
        }
    }
}