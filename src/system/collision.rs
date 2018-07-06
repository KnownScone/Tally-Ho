use ::utility::{Rect2, Rect3, penetration_vector};
use ::collision as coll;
use ::component as comp;
use comp::collider::*; 

use cgmath::{InnerSpace, Vector2, Vector3, Zero};
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
        specs::WriteStorage<'a, comp::Transform>, 
        specs::WriteStorage<'a, comp::Velocity>, 
        specs::WriteStorage<'a, comp::Collider>,
    );

    fn run(&mut self, (ent, mut tran, mut vel, mut coll): Self::SystemData) {
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
        use specs::Join;
        
        // Initialize the collider with its transform.
        for (ent, tran, mut coll, _) in (&*ent, &tran, &mut coll, &self.ins_transform).join() {
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
            let vel = tran.pos - tran.last_pos;

            let bound = if coll.sweep {
                let old_bound = coll.shape.bound(tran.last_pos);
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
        }

        self.broad_phase.for_each(|(e1, e2)| {
            let mut d1 = Vector3::zero();
            let mut d2 = Vector3::zero();
            
            {
                let c1 = coll.get(e1).unwrap();
                let c2 = coll.get(e2).unwrap();
                let t1 = tran.get(e1).unwrap();
                let t2 = tran.get(e2).unwrap();

                // This is a discrete collision, we can solve immediately.
                if !c1.sweep && !c2.sweep {
                    if let Shape::AABB(r1) = c1.shape {
                        if let Shape::AABB(r2) = c2.shape {
                            let r1 = Rect3::new(
                                t1.pos + r1.min,
                                t1.pos + r1.max,
                            );

                            let r2 = Rect3::new(
                                t2.pos + r2.min,
                                t2.pos + r2.max,
                            );

                            let pen1 = penetration_vector(r1, r2);
                            let pen2 = penetration_vector(r2, r1);

                            d1 = pen1 / 2.0;
                            d2 = pen2 / 2.0;
                        } else if let Shape::Circle {offset: o2, radius: r2, depth: ref d2} = c2.shape {
                            // TODO: Discrete AABB-Circle collision
                        }
                    }
                } else {
                    // TODO: Sweep collisions
                }
            }

            if d1 != Vector3::zero() {
                let t = tran.get_mut(e1).unwrap();
                let v = vel.get_mut(e1).unwrap();
                t.last_pos = t.pos;
                t.pos += d1;

                let mag = v.pos.magnitude();
                let norm = d1.normalize();
                v.pos = norm * mag;
            } if d2 != Vector3::zero() {
                let t = tran.get_mut(e2).unwrap();
                let v = vel.get_mut(e2).unwrap();
                t.last_pos = t.pos;
                t.pos += d2;
                
                let mag = v.pos.magnitude().abs();
                let norm = d2.normalize();
                v.pos = norm * mag;
            }

        });
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut tran_storage: specs::WriteStorage<comp::Transform> = SystemData::fetch(&res);
        self.transform_ins_read = Some(tran_storage.track_inserted());        
        self.transform_mod_read = Some(tran_storage.track_modified());        
    }
}