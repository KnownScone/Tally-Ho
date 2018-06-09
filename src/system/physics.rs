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

    fn run(&mut self, (mut tran, vel, dt): Self::SystemData) {
        use specs::Join;

        let dt = dt.0;
        
        for (mut tran, vel) in (&mut tran.restrict_mut(), &vel).join() {
            if vel.x != 0.0 || vel.y != 0.0 {
                let tran = tran.get_mut_unchecked();
                tran.x += vel.x * dt;
                tran.y += vel.y * dt;
            }
        }
    }
}