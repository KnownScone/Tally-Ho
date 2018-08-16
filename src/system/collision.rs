use ::utility::{Rect2, Rect3, penetration_vector, sweep_aabb};
use ::collision as coll;
use ::component as comp;
use ::resource as res;
use ::script::{LuaEntity, LuaWorld};
use comp::collider::*;

use std::f32;
use std::rc::Rc;
use std::cell::RefCell;

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

            self.broad_phase.update(coll.index.unwrap(), bound);
        }

        self.broad_phase.for_each(|(e1, e2)| {
            let c1 = coll.get(e1).unwrap();
            let c2 = coll.get(e2).unwrap();

            let mut new_pos1 = None;
            let mut new_pos2 = None;
            let mut new_vel1 = None;
            let mut new_vel2 = None;
            let mut collision = false;
            
            {
                let t1 = tran.get(e1).unwrap();
                let t2 = tran.get(e2).unwrap();
                let disp1 = t1.pos - t1.last_pos;
                let disp2 = t2.pos - t2.last_pos;

                match (&c1.shape, &c2.shape) {
                    // Discrete AABB-AABB collision.
                    (&Shape::AABB(r1), &Shape::AABB(r2)) 
                    if !c1.sweep && !c2.sweep => {
                        let r1 = Rect3::new(
                            t1.pos + r1.min,
                            t1.pos + r1.max,
                        );

                        let r2 = Rect3::new(
                            t2.pos + r2.min,
                            t2.pos + r2.max,
                        );

                        let pen = penetration_vector(r1, r2);

                        if relative_ne!(pen, Vector3::zero()) {
                            let d1 = pen / 2.0;
                            let d2 = -pen / 2.0;
                            new_pos1 = Some(t1.pos + d1);
                            new_pos2 = Some(t2.pos + d2);
                            
                            // TODO: Set velocity?

                            collision = true;
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
                            let mut d1 = (disp1 * t_first).map(|x| x - if relative_ne!(x, 0.0) {x.signum() * f32::EPSILON} else {0.0});

                            let time_left = 1.0 - t_first;
                            let dot = (disp1.x * norm.y + disp1.y * norm.x) * time_left;
                            let slide = Vector3::new(dot * norm.y, dot * norm.x, 0.0);

                            d1 += slide;

                            new_pos1 = Some(t1.last_pos + d1);
                            collision = true;
                        }
                        if let Some((t_first, t_last, norm)) = sweep_aabb(r2, t2.last_pos, disp2, r1, t1.last_pos, disp1) {
                            let mut d2 = (disp2 * t_first).map(|x| x - if relative_ne!(x, 0.0) {x.signum() * f32::EPSILON} else {0.0});

                            let time_left = 1.0 - t_first;
                            let dot = (disp2.x * norm.y + disp2.y * norm.x) * time_left;
                            let slide = Vector3::new(dot * norm.y, dot * norm.x, 0.0);

                            d2 += slide;

                            new_pos2 = Some(t2.last_pos + d2);
                            collision = true;
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
            }

            if let Some(pos) = new_pos1 {
                let t = tran.get_mut(e1).unwrap();
                t.pos = pos;
            }
            if let Some(new_vel) = new_vel1 {
                if let Some(v) = vel.get_mut(e1) {
                    v.pos = new_vel;
                }
            }
            if let Some(pos) = new_pos2 {
                let t = tran.get_mut(e2).unwrap();
                t.pos = pos;
            }
            if let Some(new_vel) = new_vel2 {
                if let Some(v) = vel.get_mut(e2) {
                    v.pos = new_vel;
                }
            }

            if collision {
                lazy.exec_mut(move |world| {
                    unsafe {
                        let res = &mut world.res as *mut specs::Resources;
                        if let Some(ref mutex) = world.read_resource::<res::Lua>().0 {
                            let lua = mutex.lock().unwrap();

                            let coll = world.read_storage::<comp::Collider>();

                            let (cb1, cb2) = (
                                coll.get(e1).unwrap()
                                    .on_collide.as_ref(),
                                coll.get(e2).unwrap()
                                    .on_collide.as_ref(),
                            );

                            if cb1.is_some() || cb2.is_some() {
                                if let Some(cb) = cb1.and_then(|x| lua.registry_value::<LuaFunction>(&x).ok()) {
                                    cb.call::<_, ()>((LuaWorld(res), LuaEntity(e1), LuaEntity(e2))).unwrap();
                                }
                                if let Some(cb) = cb2.and_then(|x| lua.registry_value::<LuaFunction>(&x).ok()) {
                                    cb.call::<_, ()>((LuaWorld(res), LuaEntity(e2), LuaEntity(e1))).unwrap();
                                }
                            }
                        }
                    }
                });
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