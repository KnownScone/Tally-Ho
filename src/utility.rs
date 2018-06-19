use cgmath::{Vector2, Vector3};

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect3<S> {
    pub min: Vector3<S>, 
    pub max: Vector3<S>, 
}

impl<S> Rect3<S> {
    pub fn new(min: Vector3<S>, max: Vector3<S>) -> Self {
        Rect3 {
            min,
            max
        }
    }
}