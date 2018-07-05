use cgmath::{BaseNum, Zero, Vector2, Vector3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2<S> {
    pub min: Vector2<S>, 
    pub max: Vector2<S>, 
}

impl<S: BaseNum> Rect2<S> {
    pub fn new(min: Vector2<S>, max: Vector2<S>) -> Self {
        assert!(max.x > min.x && max.y > min.y, "Min must be smaller than or equal to max.");

        Rect2 {
            min,
            max
        }
    }

    pub fn extend(self, min_z: S, max_z: S) -> Rect3<S> {
        Rect3::new(self.min.extend(min_z), self.max.extend(max_z))
    }

    pub fn is_intersecting(self, other: Self) -> bool {
        (self.min.x < other.max.x && self.max.x > other.min.x) &&
        (self.min.y < other.max.y && self.max.y > other.min.y)
    }

    pub fn minowski_difference(&self, other: Rect2<S>) -> Rect2<S> {
        Rect2 {
            min: self.min - other.min,
            max: self.max - other.max
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect3<S> {
    pub min: Vector3<S>, 
    pub max: Vector3<S>, 
}

impl<S: BaseNum> Rect3<S> {
    pub fn new(min: Vector3<S>, max: Vector3<S>) -> Self {
        assert!(max.x > min.x && max.y > min.y && max.z >= min.z, "Min must be smaller than or equal to max.");
        Rect3 {
            min,
            max
        }
    }

    pub fn truncate(self) -> Rect2<S> {
        Rect2::new(Vector2::new(self.min.x, self.min.y), Vector2::new(self.max.x, self.max.y))
    }

    pub fn is_intersecting(self, other: Self) -> bool {
        (self.min.x < other.max.x && self.max.x > other.min.x) &&
        (self.min.y < other.max.y && self.max.y > other.min.y) &&
        // : Only having the z check for equalness is some wonky shit, ya think we should just retire the ol' z-axis and return to int-based layers?
        (self.min.z <= other.max.z && self.max.z >= other.min.z)
    }

    pub fn minowski_difference(&self, other: Rect3<S>) -> Rect3<S> {
        let size = self.max - self.min;
        let other_size = other.max - other.min;

        let top_left = Vector3::new(
            self.min.x - other.min.x - other_size.x,
            self.min.y - other.min.y - other_size.y,
            self.min.z - other.min.z - other_size.z,
        );

        Rect3 {
            min: top_left,
            max: top_left + (size + other_size)
        }
    }
}

pub fn closest_bounds_point_to_point(rect: Rect3<f32>, point: Vector3<f32>) -> Vector3<f32>{
    let mut min_dist = (rect.min.x - point.x).abs();
    let mut bounds_point = Vector3::new(rect.min.x, point.y, point.z);
    
    if (rect.max.x - point.x).abs() <= min_dist {
        min_dist = (rect.max.x - point.x).abs();
        bounds_point = Vector3::new(rect.max.x, point.y, point.z);
        
    } if (rect.max.y - point.y).abs() <= min_dist {
        min_dist = (rect.max.y - point.y).abs();
        bounds_point = Vector3::new(point.x, rect.max.y, point.z);
    } if (rect.min.y - point.y).abs() <= min_dist {
        min_dist = (rect.min.y - point.y).abs();
        bounds_point = Vector3::new(point.x, rect.min.y, point.z);
    }

    bounds_point
}

pub fn penetration_vector(r1: Rect3<f32>, r2: Rect3<f32>) -> Vector3<f32> {
    let md = r2.minowski_difference(r1);
    closest_bounds_point_to_point(md, Vector3::zero())
}