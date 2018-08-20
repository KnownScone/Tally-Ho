use ::utility::{Rect2, Rect3, penetration_vector, sweep_aabb};
use ::collision as coll;
use ::component as comp;
use ::resource as res;
use ::script::{LuaEntity, LuaWorld};
use comp::collider::*;

use std::f32;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::hash_map::*;

use cgmath::{InnerSpace, ApproxEq, Vector2, Vector3, Zero};
use specs;
use rlua::{Function as LuaFunction, LightUserData, UserData, UserDataMethods, AnyUserData, Scope as LuaScope};

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
        specs::Read<'a, specs::LazyUpdate>,
    );

    fn run(&mut self, (ent, mut tran, mut vel, mut coll, lazy): Self::SystemData) {
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
        
        // Initialize the collider with its transform.
        for (ent, tran, mut coll, _) in (&*ent, &tran, &mut coll, &self.ins_transform).join() {
            let obj = coll::Object {
                bound: coll.shape.bound(tran.pos),
                entity: ent,
            };

            // Insert new object into broadphase.
            let idx = self.broad_phase.insert(obj);

            // Make sure the collider knows where it is.
            coll.index = Some(idx);
        }

        // Move the collider with its recently modified transform.
        for (ent, tran, mut coll, _) in (&*ent, &tran, &mut coll, &self.mod_transform).join() {
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
                            old_bound.rect.max.x.max(new_bound.rect.max.x),
                            old_bound.rect.max.y.max(new_bound.rect.max.y),
                            old_bound.rect.max.z.max(new_bound.rect.max.z),
                        )
                    )
                }
            } else {
                coll.shape.bound(tran.pos)
            };

            // Update the collision object on the broadphase grid.
            self.broad_phase.update(coll.index.unwrap(), bound);
        }

        // Maps swept entities to their (current) minimum time of impact and the index of the collision.
        let mut min_sweep: HashMap<specs::Entity, (f32, usize)> = HashMap::new();
        // Maps discrete entities to their (current) maxmimum displacement (to get them out of a collision) and the index of the collision.
        let mut max_disp: HashMap<specs::Entity, (Vector3<f32>, usize)> = HashMap::new();
        // List of collisions that will be resolved.
        let mut collisions: Vec<Collision> = Vec::new();

        // Loop through all the collision pairs that the broad phase has detected.
        // * There should be no "duplicates", as in the same pair of entities showing up but in the opposite order.
        self.broad_phase.for_each(|(e1, e2)| {
            // TODO: Filter collisions

            // Get the components we need.
            let c1 = coll.get(e1).unwrap();
            let c2 = coll.get(e2).unwrap();
            let t1 = tran.get(e1).unwrap();
            let t2 = tran.get(e2).unwrap();
            let disp1 = t1.pos - t1.last_pos;
            let disp2 = t2.pos - t2.last_pos;

            match (&c1.shape, &c2.shape) {
                // Discrete AABB-AABB collision.
                (&Shape::AABB(r1), &Shape::AABB(r2)) 
                if !c1.sweep && !c2.sweep => {
                    // The collider AABB in world space.
                    let r1 = Rect3::new(
                        t1.pos + r1.min,
                        t1.pos + r1.max,
                    );

                    // The collider AABB in world space.
                    let r2 = Rect3::new(
                        t2.pos + r2.min,
                        t2.pos + r2.max,
                    );

                    let pen = penetration_vector(r1, r2);

                    // If the two colliders actually penetrated eachother.
                    if relative_ne!(pen, Vector3::zero()) {
                        use cgmath::ElementWise;

                        let (abs_disp1, abs_disp2) = (disp1.map(|x| x.abs()), disp2.map(|x| x.abs()));
                        
                        let factor1 = abs_disp1.div_element_wise(abs_disp1 + abs_disp2).map(|x| if x.is_nan() {0.0} else {x});
                        let factor2 = abs_disp2.div_element_wise(abs_disp1 + abs_disp2).map(|x| if x.is_nan() {0.0} else {x});
                        
                        let d1 = pen.mul_element_wise(factor1);
                        let d2 = -pen.mul_element_wise(factor2);

                        match max_disp.entry(e1) {
                            Entry::Occupied(mut entry) => {
                                // If this disp has a magnitude greater than the current one, replace it.
                                if d1.magnitude2() > entry.get().0.magnitude2() {
                                    collisions[entry.get().1] = 
                                        Collision::Discrete(
                                            e1,
                                            e2,
                                            d1
                                        );

                                    entry.get_mut().0 = d1;
                                }
                            },
                            Entry::Vacant(entry) => {
                                collisions.push(Collision::Discrete(
                                    e1,
                                    e2,
                                    d1
                                ));
                                entry.insert((d1, collisions.len() - 1));
                            }
                        }

                        match max_disp.entry(e2) {
                            Entry::Occupied(mut entry) => {
                                // If this disp has a magnitude greater than the current one, replace it.
                                if d2.magnitude2() > entry.get().0.magnitude2() {
                                    collisions[entry.get().1] = 
                                        Collision::Discrete(
                                            e2,
                                            e1,
                                            d2
                                        );

                                    entry.get_mut().0 = d2;
                                }
                            },
                            Entry::Vacant(entry) => {
                                collisions.push(Collision::Discrete(
                                    e2,
                                    e1,
                                    d2
                                ));
                                entry.insert((d2, collisions.len() - 1));
                            }
                        }
                    }
                },
                // Discrete AABB-Circle collision.
                (&Shape::AABB(r), &Shape::Circle{offset: c_o, radius: c_r, depth: ref c_d}) 
                    | (&Shape::Circle{offset: c_o, radius: c_r, depth: ref c_d}, &Shape::AABB(r))
                if !c1.sweep && !c2.sweep => {
                    // TODO
                },
                // Discrete Circle-Circle collision.
                (&Shape::Circle{offset: c1_o, radius: c1_r, depth: ref c1_d}, &Shape::Circle{offset: c2_o, radius: c2_r, depth: ref c2_d}) 
                if !c1.sweep && !c2.sweep => {
                    // TODO
                },
                // Sweep AABB-AABB collision.
                (&Shape::AABB(r1), &Shape::AABB(r2)) 
                if c1.sweep || c2.sweep => {
                    if let Some((t_first, t_last, norm)) = sweep_aabb(r1, t1.last_pos, disp1, r2, t2.last_pos, disp2) {
                        match min_sweep.entry(e1) {
                            Entry::Occupied(mut entry) => {
                                // If this TOI (time-of-impact) is earlier than the current one, replace it.
                                if t_first < entry.get().0 {
                                    collisions[entry.get().1] = 
                                        Collision::Sweep(
                                            e1,
                                            e2,
                                            t_first,
                                            norm
                                        );

                                    entry.get_mut().0 = t_first;
                                }
                            },
                            Entry::Vacant(entry) => {
                                collisions.push(Collision::Sweep(
                                    e1,
                                    e2,
                                    t_first,
                                    norm
                                ));
                                entry.insert((t_first, collisions.len() - 1));
                            }
                        }

                        match min_sweep.entry(e2) {
                            Entry::Occupied(mut entry) => {
                                // If this TOI (time-of-impact) is earlier than the current one, replace it.
                                if t_first < entry.get().0 {
                                    collisions[entry.get().1] = 
                                        Collision::Sweep(
                                            e2,
                                            e1,
                                            t_first,
                                            norm
                                        );

                                    entry.get_mut().0 = t_first;
                                }
                            },
                            Entry::Vacant(entry) => {
                                collisions.push(Collision::Sweep(
                                    e2,
                                    e1,
                                    t_first,
                                    norm
                                ));
                                entry.insert((t_first, collisions.len() - 1));
                            }
                        }

                    }
                },
                // Sweep AABB-Circle collision.
                (&Shape::AABB(r), &Shape::Circle{offset: c_o, radius: c_r, depth: ref c_d}) 
                    | (&Shape::Circle{offset: c_o, radius: c_r, depth: ref c_d}, &Shape::AABB(r))
                if c1.sweep || c2.sweep => {
                    // TODO
                }, 
                // Sweep Circle-Circle collision.
                (&Shape::Circle{offset: c1_o, radius: c1_r, depth: ref c1_d}, &Shape::Circle{offset: c2_o, radius: c2_r, depth: ref c2_d}) 
                if c1.sweep || c2.sweep => {
                    // TODO
                },
                _ => ()
            }
        });

        for collision in collisions {
            match collision {
                Collision::Sweep(ent, other, toi, norm) => {
                    let other_toi = min_sweep[&other].0;

                    // Double check if these entities actually collide. Before, for example, hitting another object.
                    if relative_eq!(toi, other_toi) {
                        let t = tran.get_mut(ent).unwrap();
                        let disp = t.pos - t.last_pos;

                        let mut new_disp = (disp * toi).map(|x| x - if relative_ne!(x, 0.0) {x.signum() * f32::EPSILON} else {0.0});

                        /* Slide mechanics:
                            With what remaining velocity an object has, slide it along the surface it collided with
                            in the direction that the object approached it (e.g. if going diagonally up, it slides up).
                        */
                        let time_left = 1.0 - toi;
                        let dot = (disp.x * norm.y + disp.y * norm.x) * time_left;
                        let slide = Vector3::new(dot * norm.y, dot * norm.x, 0.0);
                        new_disp += slide;

                        t.pos = t.last_pos + new_disp;

                        lazy.exec_mut(move |world| {
                            let res = &mut world.res as *mut specs::Resources;

                            if let Some(ref mutex) = world.read_resource::<res::Script>().0 {
                                let script = mutex.lock().unwrap();

                                let coll = world.read_storage::<comp::Collider>();
                                
                                if let Some(cb) = coll.get(ent).unwrap().on_collide.as_ref() {
                                    if let Some(func) = script.registry_value::<LuaFunction>(&cb).ok() {
                                        func.call::<_, ()>((LuaWorld(res), LuaEntity(ent), LuaEntity(other))).unwrap();
                                    }
                                }
                            }
                        });
                    }
                },
                Collision::Discrete(ent, other, disp) => {
                    let t = tran.get_mut(ent).unwrap();
                    t.pos += disp;

                    lazy.exec_mut(move |world| {
                        let res = &mut world.res as *mut specs::Resources;

                        if let Some(ref mutex) = world.read_resource::<res::Script>().0 {
                            let script = mutex.lock().unwrap();

                            let coll = world.read_storage::<comp::Collider>();
                            
                            if let Some(cb) = coll.get(ent).unwrap().on_collide.as_ref() {
                                if let Some(func) = script.registry_value::<LuaFunction>(&cb).ok() {
                                    func.call::<_, ()>((LuaWorld(res), LuaEntity(ent), LuaEntity(other))).unwrap();
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut tran_storage: specs::WriteStorage<comp::Transform> = SystemData::fetch(&res);
        self.transform_ins_read = Some(tran_storage.track_inserted());        
        self.transform_mod_read = Some(tran_storage.track_modified());        
    }
}

#[derive(Debug)]
enum Collision {
    Sweep(specs::Entity, specs::Entity, f32, Vector3<f32>),
    Discrete(specs::Entity, specs::Entity, Vector3<f32>),
}