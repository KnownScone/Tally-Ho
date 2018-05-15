use specs;

#[derive(Debug)]
pub struct Velocity {
    pub x: f32,
    pub y: f32
}

impl specs::Component for Velocity {
    type Storage = specs::VecStorage<Self>;
}