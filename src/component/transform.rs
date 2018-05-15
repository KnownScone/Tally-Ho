use specs;

#[derive(Debug)]
pub struct Transform {
    pub x: f32,
    pub y: f32
}

impl specs::Component for Transform {
    type Storage = specs::VecStorage<Self>;
}