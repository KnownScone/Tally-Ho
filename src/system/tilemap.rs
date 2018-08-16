use ::{vs, Vertex};
use ::component as comp;
use ::resource as res;
use ::parse;
use ::utility::{Rect2, Rect3};

use std::sync::Arc;

use vulkano as vk;
use vk::descriptor::descriptor_set::FixedSizeDescriptorSetsPool;
use vk::buffer::CpuBufferPool;
use cgmath::{Vector2, Vector3, Matrix4, Zero};
use specs;

pub struct TileMapSystem;

impl<'a> specs::System<'a> for TileMapSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, comp::TileMap>,
        specs::WriteStorage<'a, comp::RenderStrip>,
        specs::WriteStorage<'a, comp::CollisionStrip>,
    );

    fn run(&mut self, (ents, mut map, mut rndr_str, mut coll_str): Self::SystemData) {
        use std::collections::hash_map::*;
        use specs::Join;

        for (ent, mut map) in (&*ents, &mut map).join() {
            // If we need to load the map.
            if let Some(load) = map.load.take() {
                let mut render: HashMap<Vector3<u32>, comp::RenderStrip> = HashMap::new();
                let mut collision: HashMap<Vector3<u32>, comp::CollisionStrip> = HashMap::new();

                for chunk in load.chunks.iter() {
                    for layer in chunk.layers.iter() {
                        match layer.property {
                            parse::LayerProperty::TileIndex => {
                                for (idx, strip) in layer.strips.iter().enumerate() {
                                    let strip_pos = Vector3::new(
                                        idx as u32 % chunk.dimensions.x,
                                        (idx as f32 / chunk.dimensions.x as f32).floor() as u32,
                                        chunk.pos.z
                                    );

                                    let data: Vec<Option<Rect2<f32>>> = strip.iter()
                                        .map(|tex_idx| {
                                            let subtex_dims = Vector2::new(
                                                1.0 / map.tex_dims().x as f32,
                                                1.0 / map.tex_dims().y as f32
                                            ); 

                                            assert!(
                                                (*tex_idx as u32) < map.tex_dims().x * map.tex_dims().y, 
                                                "Texture index ({}) is too large (should be less than {}).", tex_idx, map.tex_dims().x * map.tex_dims().y
                                            );

                                            let pos = Vector2::new(
                                                (*tex_idx as f32 % map.tex_dims().x as f32) * subtex_dims.x,
                                                (*tex_idx as f32 / map.tex_dims().x as f32).floor() * subtex_dims.y,
                                            );

                                            Some(Rect2::new(
                                                Vector2::new(pos.x, pos.y),
                                                Vector2::new(pos.x + subtex_dims.x, pos.y + subtex_dims.y)
                                            ))
                                        })
                                    .collect();

                                    let mut uvs = [None; comp::tilemap::STRIP_LENGTH];
                                    uvs.copy_from_slice(&data[..]);

                                    match render.entry(strip_pos) {
                                        // If the strip already exists, set its blocking data.
                                        Entry::Occupied(mut entry) => {
                                            entry.get_mut().set_uvs(uvs);
                                        },
                                        // Otherwise, insert a new strip with this blocking data.
                                        Entry::Vacant(entry) => {
                                            entry.insert(comp::RenderStrip::new(
                                                ent,
                                                strip_pos,
                                                uvs,
                                            ));
                                        }
                                    }
                                }
                            },

                            parse::LayerProperty::Blocking => {
                                for (idx, strip) in layer.strips.iter().enumerate() {
                                    let strip_pos = Vector3::new(
                                        idx as u32 % chunk.dimensions.x,
                                        (idx as f32 / chunk.dimensions.x as f32).floor() as u32,
                                        chunk.pos.z
                                    );

                                    let data: Vec<bool> = strip.iter()
                                        // If the number is not 0, then the tile is blocking.
                                        .map(|num| *num != 0)
                                    .collect();

                                    let mut blocking = [false; comp::tilemap::STRIP_LENGTH];
                                    blocking.copy_from_slice(&data[..]);

                                    match collision.entry(strip_pos) {
                                        // If the strip already exists, set its blocking data.
                                        Entry::Occupied(mut entry) => {
                                            entry.get_mut().blocking = blocking;
                                        },
                                        // Otherwise, insert a new strip with this blocking data.
                                        Entry::Vacant(entry) => {
                                            entry.insert(comp::CollisionStrip::new(
                                                ent,
                                                strip_pos,
                                                blocking,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                for (pos, strip) in render.into_iter() {
                    ents.build_entity()
                        .with(strip, &mut rndr_str)
                    .build();
                }

                for (pos, strip) in collision.into_iter() {
                    ents.build_entity()
                        .with(strip, &mut coll_str)
                    .build();
                }
            }
        }
    }
}

pub struct TileMapCollisionSystem {
    collision_strip_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    collision_strip_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    ins_collision_strip: specs::BitSet,
    mod_collision_strip: specs::BitSet,
}

impl TileMapCollisionSystem {
    pub fn new(
    ) -> TileMapCollisionSystem {
        TileMapCollisionSystem {
            collision_strip_ins_read: None,
            collision_strip_mod_read: None,
            ins_collision_strip: specs::BitSet::new(),
            mod_collision_strip: specs::BitSet::new(),
        }
    }
}

impl<'a> specs::System<'a> for TileMapCollisionSystem {
    type SystemData = (
        specs::Entities<'a>, 
        specs::WriteStorage<'a, comp::Transform>,
        specs::WriteStorage<'a, comp::TileMap>,
        specs::WriteStorage<'a, comp::Collider>,
        specs::WriteStorage<'a, comp::CollisionStrip>,
    );

    fn run(&mut self, (ents, mut trans, mut map, mut colls, mut strip): Self::SystemData) {
        use specs::Join;

        self.ins_collision_strip.clear();
        self.mod_collision_strip.clear();
        
        strip.populate_inserted(&mut self.collision_strip_ins_read.as_mut().unwrap(), &mut self.ins_collision_strip);
        strip.populate_modified(&mut self.collision_strip_mod_read.as_mut().unwrap(), &mut self.mod_collision_strip);

        for (mut strip, _) in (&mut strip, &self.ins_collision_strip).join() {
            let map = map.get(strip.tile_map()).unwrap();
            
            let strip_pos = Vector3::new(
                strip.pos().x as f32 * comp::tilemap::STRIP_LENGTH as f32 * map.tile_dims().x,
                strip.pos().y as f32 * comp::tilemap::STRIP_LENGTH as f32 * map.tile_dims().y,
                strip.pos().z as f32 * comp::tilemap::STRIP_LENGTH as f32 * map.tile_dims().z
            );
            
            for (idx, tile) in strip.blocking.iter().enumerate() {
                // If this tile does not block, try the next one.
                if !*tile {
                    continue;
                }

                let tile_pos = Vector3::new(
                    idx as f32 * map.tile_dims().x,
                    0.0,
                    0.0
                );

                let coll = comp::Collider::new(
                    comp::collider::Shape::AABB(
                        Rect3::new(
                            Vector3::zero(),
                            map.tile_dims(),
                        )
                    ), 
                    false, 
                    None
                );

                let tran = comp::Transform::new(
                    strip_pos + tile_pos,
                );

                let e = ents.build_entity()
                    .with(tran, &mut trans)
                    .with(coll, &mut colls)
                .build();

                strip.colliders.push(e);
            }
        }

        // TODO: When a strip has been modified
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut coll_strip_storage: specs::WriteStorage<comp::CollisionStrip> = SystemData::fetch(&res);
        self.collision_strip_ins_read = Some(coll_strip_storage.track_inserted());
        self.collision_strip_mod_read = Some(coll_strip_storage.track_modified());
    }
}

pub struct TileMapRenderSystem {
    render_strip_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    render_strip_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    ins_render_strip: specs::BitSet,
    mod_render_strip: specs::BitSet,
}

impl TileMapRenderSystem {
    pub fn new() -> TileMapRenderSystem {
        TileMapRenderSystem {
            render_strip_ins_read: None,
            render_strip_mod_read: None,
            ins_render_strip: specs::BitSet::new(),
            mod_render_strip: specs::BitSet::new(),
        }
    }
}

impl<'a> specs::System<'a> for TileMapRenderSystem {
    type SystemData = (
        specs::Read<'a, res::Queue>,
        specs::Write<'a, res::SortedRender>,
        specs::Entities<'a>,
        specs::ReadStorage<'a, comp::TileMap>,
        specs::WriteStorage<'a, comp::RenderStrip>,
    );

    fn run(&mut self, (queue, mut sort_rndr, ent, map, mut strip): Self::SystemData) {
        use specs::Join;

        let queue = queue.0.as_ref().unwrap();

        // Get the components in need of initialization or an update.
        self.ins_render_strip.clear();
        self.mod_render_strip.clear();
        
        strip.populate_inserted(&mut self.render_strip_ins_read.as_mut().unwrap(), &mut self.ins_render_strip);
        strip.populate_modified(&mut self.render_strip_mod_read.as_mut().unwrap(), &mut self.mod_render_strip);

        for (ent, mut strip, _) in (&*ent, &mut strip, &self.ins_render_strip | &self.mod_render_strip).join() {
            // Create the vertex and index buffers for all the strips without them.
            if strip.vertex_buf.is_none() || strip.index_buf.is_none() {
                let map = map.get(strip.tile_map()).unwrap();

                let world_pos = Vector3::new(
                    (strip.pos().x * comp::tilemap::STRIP_LENGTH as u32) as f32 * map.tile_dims().x,
                    strip.pos().y as f32 * map.tile_dims().y,
                    strip.pos().z as f32 * map.tile_dims().z
                );

                let vertex_data: Vec<_> = strip.uvs().iter().filter_map(|x| *x).enumerate()
                    .flat_map(|(idx, uv)| {
                        let local_pos = Vector3::new(
                            idx as f32 * map.tile_dims().x,
                            0.0,
                            0.0
                        );

                        vec![
                            Vertex {
                                position: (world_pos + local_pos).into(),
                                uv: [uv.min.x, uv.min.y]
                            },
                            Vertex {
                                position: (world_pos + local_pos + Vector3::new(map.tile_dims().x, 0.0, 0.0)).into(),
                                uv: [uv.max.x, uv.min.y]
                            },
                            Vertex {
                                position: (world_pos + local_pos + Vector3::new(0.0, map.tile_dims().y, 0.0)).into(),
                                uv: [uv.min.x, uv.max.y]
                            },
                            Vertex {
                                position: (world_pos + local_pos + Vector3::new(map.tile_dims().x, map.tile_dims().y, 0.0)).into(),
                                uv: [uv.max.x, uv.max.y]
                            }
                        ]
                    })
                .collect();

                let index_data: Vec<_> = strip.uvs().iter().filter_map(|x| *x).enumerate()
                    .flat_map(|(idx, _)| {
                        let i = idx as u32 * 4;
                        vec![
                            i, i + 1, i + 2,
                            i + 1, i + 2, i + 3
                        ]
                    })
                .collect();

                let (vertex_buf, _) = vk::buffer::ImmutableBuffer::from_iter(
                    vertex_data.iter().cloned(),
                    vk::buffer::BufferUsage::vertex_buffer(),
                    queue.clone()
                ).expect("Couldn't create vertex buffer");

                let (index_buf, _) = vk::buffer::ImmutableBuffer::from_iter(
                    index_data.iter().cloned(),
                    vk::buffer::BufferUsage::index_buffer(),
                    queue.clone()
                ).expect("Couldn't create index buffer");

                strip.vertex_buf = Some(vertex_buf);
                strip.index_buf = Some(index_buf);
                
                // If this strip has just being initialized, add it to the sorted renders.
                if self.ins_render_strip.contains(ent.id()) {
                    sort_rndr.ids.push(res::RenderId::TileStrip(ent));
                }

                // After updating the strip's data, the strip needs to be resorted.
                sort_rndr.need_sort = true;
            }
        }
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut rndr_strip_storage: specs::WriteStorage<comp::RenderStrip> = SystemData::fetch(&res);
        self.render_strip_ins_read = Some(rndr_strip_storage.track_inserted());
        self.render_strip_mod_read = Some(rndr_strip_storage.track_modified());
    }
}