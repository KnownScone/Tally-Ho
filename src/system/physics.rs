use ::component as comp;

use specs;

pub struct VelocitySystem;

impl<'a> specs::System<'a> for VelocitySystem {
    type SystemData = (specs::WriteStorage<'a, comp::Transform>, specs::ReadStorage<'a, comp::Velocity>);

    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        use specs::Join;

        for (pos, vel) in (&mut pos, &vel).join() {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }
}