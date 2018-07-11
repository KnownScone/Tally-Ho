use std::f32;

use cgmath::{BaseNum, Array, Zero, One, Vector2, Vector3, InnerSpace};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2<S> {
    pub min: Vector2<S>, 
    pub max: Vector2<S>, 
}

impl<S: BaseNum> Rect2<S> {
    pub fn new(min: Vector2<S>, max: Vector2<S>) -> Self {
        if max.x < min.x && max.y < min.y {
            warn!("Min must be smaller than max.");
        }

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
        let size = self.max - self.min;
        let other_size = other.max - other.min;

        let top_left = Vector2::new(
            self.min.x - other.min.x - other_size.x,
            self.min.y - other.min.y - other_size.y,
        );

        Rect2 {
            min: top_left,
            max: top_left + (size + other_size)
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
        if max.x < min.x && max.y < min.y && max.z < min.z {
            warn!("Min must be smaller than max.");
        }

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
        (self.min.z < other.max.z && self.max.z > other.min.z)
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
    
    } if (rect.max.z - point.z).abs() <= min_dist {
        min_dist = (rect.max.z - point.z).abs();
        bounds_point = Vector3::new(point.x, point.y, rect.max.z);
    } if (rect.min.z - point.z).abs() <= min_dist {
        min_dist = (rect.min.z - point.z).abs();
        bounds_point = Vector3::new(point.x, point.y, rect.min.z);
    }

    bounds_point
}

pub fn penetration_vector(r1: Rect3<f32>, r2: Rect3<f32>) -> Vector3<f32> {
    let md = r2.minowski_difference(r1);
    closest_bounds_point_to_point(md, Vector3::zero())
}

pub fn sweep_aabb(aabb1: Rect3<f32>, pos1: Vector3<f32>, disp1: Vector3<f32>, aabb2: Rect3<f32>, pos2: Vector3<f32>, disp2: Vector3<f32>) -> Option<(f32, f32)> {
    let aabb1 = Rect3::new(
        pos1 + aabb1.min,
        pos1 + aabb1.max,
    );
    let aabb2 = Rect3::new(
        pos2 + aabb2.min,
        pos2 + aabb2.max,
    );
    // Use relative velocity, essentially treating aabb1 as stationary.
    let v = disp2 - disp1;

    // Initialize times of first and last contact
    let mut t_first = 0.0;
    let mut t_last = 1.0;
    
    // For each axis, determine times of first and last contact, if any
    for i in 0..3 {
        if v[i] < 0.0 {
            if aabb2.max[i] < aabb1.min[i] { return None; } // Nonintersecting and moving apart
            if aabb1.max[i] < aabb2.min[i] { t_first = ((aabb1.max[i] - aabb2.min[i]) / v[i]).max(t_first); }
            if aabb2.max[i] > aabb1.min[i] { t_last  = ((aabb1.min[i] - aabb2.max[i]) / v[i]).min(t_last); }
        }
        if v[i] > 0.0 {
            if aabb2.min[i] > aabb1.max[i] { return None; } // Nonintersecting and moving apart
            if aabb2.max[i] < aabb1.min[i] { t_first = ((aabb1.min[i] - aabb2.max[i]) / v[i]).max(t_first); }
            if aabb1.max[i] > aabb2.min[i] { t_last = ((aabb1.max[i] - aabb2.min[i]) / v[i]).min(t_last); }
        }

        // No overlap possible if time of first contact occurs after time of last contact
        if t_first > t_last { return None; };
    }
    
    Some((t_first, t_last))
}

#[test]
fn test_sweep_aabb() {
    let aabb = Rect3::new(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 1.0),
    );

    let pos1 = Vector3::new(-3.0, 0.0, 0.0);
    let disp1 = Vector3::new(6.0, 0.0, 0.0);
    
    let pos2 = Vector3::new(3.0, 0.0, 0.0);
    let disp2 = Vector3::new(-6.0, 0.0, 0.0);

    let (t_first, t_last) = sweep_aabb(aabb, pos1, disp1, aabb, pos2, disp2)
        .expect("No hit");

    assert_eq!((t_first + t_last) / 2.0, 0.5);
}