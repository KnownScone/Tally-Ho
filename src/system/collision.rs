use ::utility::{Rect2, Rect3};
use ::collision as coll;
use ::component as comp;

use cgmath::{Vector2, Vector3, Zero};
use specs;

pub struct CollisionSystem {
    transform_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    transform_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    ins_transform: specs::BitSet,
    mod_transform: specs::BitSet,

    broad_phase: coll::BroadPhase,
}

impl CollisionSystem {
    pub fn new() -> Self {
        CollisionSystem {
            transform_ins_read: None,
            transform_mod_read: None,
            ins_transform: specs::BitSet::new(),
            mod_transform: specs::BitSet::new(),
            broad_phase: coll::BroadPhase::new(),
        }
    }
}

impl<'a> specs::System<'a> for CollisionSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, comp::Transform>, 
        specs::WriteStorage<'a, comp::Collider>,
    );

    fn run(&mut self, (ent, tran, mut coll): Self::SystemData) {
        use specs::Join;

        /* NOTE:
            Entities with collider components won't participate in 
            collision until it has a transform component.
        */

        // Get the components in need of initialization or an update.
        self.ins_transform.clear();
        self.mod_transform.clear();
        
        tran.populate_inserted(&mut self.transform_ins_read.as_mut().unwrap(), &mut self.ins_transform);
        tran.populate_modified(&mut self.transform_mod_read.as_mut().unwrap(), &mut self.mod_transform);
        
        // Broad phase

        // Initialize the collider with its transform.
        for (ent, tran, mut coll, _) in (&*ent, &tran, &mut coll, &self.ins_transform).join() {
            // For when the transform is modified (we can get the velocity via this).
            coll.last_pos = tran.pos;

            let obj = coll::Object {
                bound: coll.shape.bound(tran.pos),
                entity: ent,
            };

            // Insert new object into broadphase.
            let idx = self.broad_phase.insert(obj);

            coll.index = Some(idx);
        }

        // Move the collider with its recently modified transform.
        for (ent, tran, mut coll, _) in (&*ent, &tran, &mut coll, &self.mod_transform).join() {
            let vel = tran.pos - coll.last_pos;

            let bound = if coll.sweep {
                let old_bound = coll.bound.as_ref().unwrap();
                let new_bound = coll.shape.bound(tran.pos);

                // Get a rect that encompasses both the new and old bounds (a "swept" bound).
                comp::collider::Bound {
                    rect: Rect3::new(
                        Vector3::new(
                            old_bound.rect.min.x.min(new_bound.rect.min.x),
                            old_bound.rect.min.y.min(new_bound.rect.min.y),
                            old_bound.rect.min.z.min(new_bound.rect.min.z),
                        ),
                        Vector3::new(
                            old_bound.rect.max.x.min(new_bound.rect.max.x),
                            old_bound.rect.max.y.min(new_bound.rect.max.y),
                            old_bound.rect.max.z.min(new_bound.rect.max.z),
                        )
                    )
                }
            } else {
                coll.shape.bound(tran.pos)
            };

            self.broad_phase.update(coll.index.unwrap(), bound);

            // For next update (we can get the velocity via this).
            coll.last_pos = tran.pos;
        }

        // TODO: Narrow phase
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut tran_storage: specs::WriteStorage<comp::Transform> = SystemData::fetch(&res);
        self.transform_ins_read = Some(tran_storage.track_inserted());        
        self.transform_mod_read = Some(tran_storage.track_modified());        
    }
}