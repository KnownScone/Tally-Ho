use ::component as comp;
use ::resource as res;

use cgmath::{Zero, Vector3};
use specs;

// System meant to be executed at the end of a tick; maintains transforms.
pub struct TransformSystem;

impl<'a> specs::System<'a> for TransformSystem {
    type SystemData = specs::WriteStorage<'a, comp::Transform>;

    fn run(&mut self, mut tran: Self::SystemData) {
        use specs::Join;

        for mut tran in (&mut tran.restrict_mut()).join() {
            if relative_ne!(tran.get_unchecked().last_pos, tran.get_unchecked().pos) {
                let tran = tran.get_mut_unchecked();
                tran.last_pos = tran.pos;
            }
        }
    }
}