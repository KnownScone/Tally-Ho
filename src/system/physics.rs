use ::component as comp;
use ::resource as res;

use specs;

pub struct VelocitySystem;

impl<'a> specs::System<'a> for VelocitySystem {
    type SystemData = (
        specs::WriteStorage<'a, comp::Transform>, 
        specs::ReadStorage<'a, comp::Velocity>,
        specs::Read<'a, res::DeltaTime>
    );

    fn run(&mut self, (mut pos, vel, dt): Self::SystemData) {
        use specs::Join;

        let dt = dt.0.as_secs() as f32 + dt.0.subsec_nanos() as f32 / 1_000_000_000.0;
        
        for (mut pos, vel) in (&mut pos.restrict_mut(), &vel).join() {
            
            if vel.x != 0.0 || vel.y != 0.0 {
                let pos = pos.get_mut_unchecked();
                pos.x += vel.x * dt;
                pos.y += vel.y * dt;
            }
        }
    }
}