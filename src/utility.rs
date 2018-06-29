use cgmath::{BaseNum, Vector2, Vector3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2<S> {
    pub min: Vector2<S>, 
    pub max: Vector2<S>, 
}

impl<S: BaseNum> Rect2<S> {
    pub fn new(min: Vector2<S>, max: Vector2<S>) -> Self {
        assert!(max.x >= min.x && max.y >= min.y, "Min must be smaller than or equal to max.");

        Rect2 {
            min,
            max
        }
    }

    pub fn extend(self, min_z: S, max_z: S) -> Rect3<S> {
        Rect3::new(self.min.extend(min_z), self.max.extend(max_z))
    }

    pub fn is_intersecting(self, other: Self) -> bool {
        (self.min.x <= other.max.x && self.max.x >= other.min.x) &&
        (self.min.y <= other.max.y && self.max.y >= other.min.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect3<S> {
    pub min: Vector3<S>, 
    pub max: Vector3<S>, 
}

impl<S: BaseNum> Rect3<S> {
    pub fn new(min: Vector3<S>, max: Vector3<S>) -> Self {
        assert!(max.x >= min.x && max.y >= min.y && max.z >= min.z, "Min must be smaller than or equal to max.");
        Rect3 {
            min,
            max
        }
    }

    pub fn truncate(self) -> Rect2<S> {
        Rect2::new(Vector2::new(self.min.x, self.min.y), Vector2::new(self.max.x, self.max.y))
    }

    pub fn is_intersecting(self, other: Self) -> bool {
        (self.min.x <= other.max.x && self.max.x >= other.min.x) &&
        (self.min.y <= other.max.y && self.max.y >= other.min.y)
    }
}