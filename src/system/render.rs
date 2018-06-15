use ::{vs, fs, Vertex};
use ::component as comp;
use ::resource as res;

use std::sync::Arc;
use std::sync::mpsc;

use vulkano as vk;
use vk::buffer::{CpuBufferPool};
use vk::descriptor::descriptor_set::{FixedSizeDescriptorSetsPool};
use vk::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder};
use cgmath::{Matrix4, Vector3};
use dmsort;
use specs;

pub enum RenderId {
    Sprite(specs::Entity),
    TileStrip(specs::Entity, usize)
}

pub struct RenderSystem<L> {
    pipeline: Arc<L>,
    
    instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
    instance_buf: CpuBufferPool<vs::ty::Instance>,

    transform_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    transform_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    inserted_transform: specs::BitSet,
    modified_transform: specs::BitSet,

    tile_map_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    tile_map_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    sprite_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    sprite_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    inserted_render: specs::BitSet,
    modified_render: specs::BitSet,

    sorted: Vec<RenderId>,

    cmd_buf_tx: mpsc::Sender<AutoCommandBuffer>
}

impl<L> RenderSystem<L> 
where
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    pub fn new(
        pipeline: Arc<L>,
        instance_buf: CpuBufferPool<vs::ty::Instance>,
    ) -> (RenderSystem<L>, mpsc::Receiver<AutoCommandBuffer>) {
        let (tx, rx) = mpsc::channel();

        (RenderSystem {
            pipeline: pipeline.clone(),
            instance_sets: FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0),
            instance_buf,
            transform_ins_read: None,
            transform_mod_read: None,
            inserted_transform: specs::BitSet::new(),
            modified_transform: specs::BitSet::new(),
            tile_map_ins_read: None,
            tile_map_mod_read: None,
            sprite_ins_read: None,
            sprite_mod_read: None,
            inserted_render: specs::BitSet::new(),
            modified_render: specs::BitSet::new(),
            sorted: Vec::new(),
            cmd_buf_tx: tx
        },
        rx)
    }
}

impl<'a, L> specs::System<'a> for RenderSystem<L> 
where 
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    type SystemData = (
        specs::Read<'a, res::Device>,
        specs::Read<'a, res::Queue>,
        specs::Read<'a, res::Framebuffer>,
        specs::Read<'a, res::DynamicState>,
        specs::Read<'a, res::ViewProjectionSet>,
        specs::Read<'a, res::TextureSet>,
        specs::Entities<'a>,
        specs::WriteStorage<'a, comp::Sprite>,
        specs::WriteStorage<'a, comp::TileMap>,
        specs::ReadStorage<'a, comp::Transform>
    );

    fn run(&mut self, (device, queue, framebuffer, state, view_proj, tex_set, ent, mut sprite, mut map, tran): Self::SystemData) {
        use specs::Join;

        let queue = queue.0.as_ref().unwrap();
        let device = device.0.as_ref().unwrap();
        let framebuffer = framebuffer.0.as_ref().unwrap();
        let state = state.0.as_ref().unwrap();
        let view_proj = view_proj.0.as_ref().unwrap();
        let tex_set = tex_set.0.as_ref().unwrap();

        // Get the components in need of initialization or an update
        self.inserted_transform.clear();
        self.modified_transform.clear();

        self.inserted_render.clear();
        self.modified_render.clear();
        
        tran.populate_inserted(&mut self.transform_ins_read.as_mut().unwrap(), &mut self.inserted_transform);
        tran.populate_modified(&mut self.transform_mod_read.as_mut().unwrap(), &mut self.modified_transform);

        sprite.populate_inserted(&mut self.sprite_ins_read.as_mut().unwrap(), &mut self.inserted_render);
        sprite.populate_modified(&mut self.sprite_mod_read.as_mut().unwrap(), &mut self.modified_render);

        map.populate_inserted(&mut self.tile_map_ins_read.as_mut().unwrap(), &mut self.inserted_render);
        map.populate_modified(&mut self.tile_map_mod_read.as_mut().unwrap(), &mut self.modified_render);

        let mut need_sort = false;

        for (ent, mut spr, _) in (&*ent, &mut sprite, &self.inserted_render | &self.modified_render).join() {
            let vertex_data = vec![
                Vertex {
                    position: [spr.bounds.min.x, spr.bounds.min.y, 0.0],
                    uv: [spr.uv.min.x, spr.uv.min.y]
                },
                Vertex {
                    position: [spr.bounds.max.x, spr.bounds.min.y, 0.0],
                    uv: [spr.uv.max.x, spr.uv.min.y]
                },
                Vertex {
                    position: [spr.bounds.min.x, spr.bounds.max.y, 0.0],
                    uv: [spr.uv.min.x, spr.uv.max.y]
                },
                Vertex {
                    position: [spr.bounds.max.x, spr.bounds.max.y, 0.0],
                    uv: [spr.uv.max.x, spr.uv.max.y]
                }
            ];

            let (vertex_buf, _) = vk::buffer::ImmutableBuffer::from_iter(
                vertex_data.iter().cloned(),
                vk::buffer::BufferUsage::vertex_buffer(),
                queue.clone()
            ).expect("Couldn't create vertex buffer");

            spr.vertex_buf = Some(vertex_buf);

            let index_data = vec![
                0, 1, 2,
                1, 2, 3
            ];

            let (index_buf, _) = vk::buffer::ImmutableBuffer::from_iter(
                index_data.iter().cloned(),
                vk::buffer::BufferUsage::index_buffer(),
                queue.clone()
            ).expect("Couldn't create vertex buffer");

            spr.index_buf = Some(index_buf);

            if self.inserted_transform.contains(ent.id()) {
                self.sorted.push(RenderId::Sprite(ent));
            }
        }

        for (ent, mut map, _) in (&*ent, &mut map, &self.inserted_render | &self.modified_render).join() {
            let dims = map.tile_dims;
            for (idx, strip) in map.strips.iter_mut().filter(|x| x.vertex_buf.is_none() || x.index_buf.is_none()).enumerate() {
                let world_pos = Vector3::new(
                    (strip.pos().x * comp::tilemap::STRIP_LENGTH) as f32 * dims.x,
                    strip.pos().y as f32 * dims.y,
                    strip.pos().z as f32 * dims.z
                );

                let vertex_data: Vec<_> = strip.uvs().iter().filter_map(|x| *x).enumerate()
                    .flat_map(|(idx, uv)| {
                        let local_pos = Vector3::new(
                            idx as f32 * dims.x,
                            0.0,
                            0.0
                        );

                        vec![
                            Vertex {
                                position: (world_pos + local_pos).into(),
                                uv: [uv.min.x, uv.min.y]
                            },
                            Vertex {
                                position: (world_pos + local_pos + Vector3::new(dims.x, 0.0, 0.0)).into(),
                                uv: [uv.max.x, uv.min.y]
                            },
                            Vertex {
                                position: (world_pos + local_pos + Vector3::new(0.0, dims.y, 0.0)).into(),
                                uv: [uv.min.x, uv.max.y]
                            },
                            Vertex {
                                position: (world_pos + local_pos + Vector3::new(dims.x, dims.y, 0.0)).into(),
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

                if self.inserted_transform.contains(ent.id()) {
                    self.sorted.push(RenderId::TileStrip(ent, idx));
                }
            }
        }

        for (ent, tran, _) in (&*ent, &tran, &self.inserted_transform | &self.modified_transform).join() {
            let map = map.get_mut(ent);
            let sprite = sprite.get_mut(ent);
            
            if map.is_none() && sprite.is_none() {
                continue;
            }

            let instance_data = vs::ty::Instance {
                transform: Matrix4::from_translation(tran.pos).into(),
            };

            let instance_subbuf = self.instance_buf.next(instance_data)
                .expect("Couldn't build instance sub-buffer");

            // Creates a descriptor set with the newly-allocated subbuffer (containing our instance data).
            let set = Arc::new(
                self.instance_sets.next()
                    .add_buffer(instance_subbuf).unwrap()
                    .build().unwrap()
            );

            if let Some(map) = map {
                map.instance_set = Some(set);
            } else if let Some(sprite) = sprite {
                sprite.instance_set = Some(set);
            }

            need_sort = true;
        }

        if need_sort {
            dmsort::sort_by(&mut self.sorted, |id1, id2| {
                let get_values = |id: &RenderId| {
                    match *id {
                        RenderId::Sprite(e) => {
                            let t = tran.get(e).unwrap();
                            let s = sprite.get(e).unwrap();
                            let b = t.pos.y + s.bounds.max.y;

                            (t, b)
                        },
                        RenderId::TileStrip(e, idx) => {
                            let t = tran.get(e).unwrap();
                            let m = map.get(e).unwrap();
                            let s = &m.strips[idx];
                            let b = t.pos.y + (m.tile_dims.y * (s.pos().y + 1) as f32);

                            (t, b)
                        }
                    }
                };

                let (t1, b1) = get_values(id1);
                let (t2, b2) = get_values(id2);

                use std::cmp::Ordering;
                let order = t1.pos.z.partial_cmp(&t2.pos.z).unwrap();

                match order {
                    Ordering::Less => order,
                    Ordering::Greater => order,
                    Ordering::Equal => b1.partial_cmp(&b2).unwrap()
                }
            });
        }

        // Holds the list of commands that are going to be executed.
        // * The only queues able to execute the command buffer are the ones of the family passed to the constructor.
        let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
            .begin_render_pass(
                framebuffer.clone(), 
                false, 
                // Clear values for the attachments marked 'load: Clear' on the render pass.
                vec![
                    // The color used to clear the screen.
                    [1.0, 1.0, 1.0, 1.0].into()
                ]
            ).unwrap();

        for id in self.sorted.iter() {
            match *id {
                RenderId::Sprite(e) => {
                    let sprite = sprite.get(e).unwrap();

                    let instance_set = sprite.instance_set.as_ref().unwrap();
                    let v_buf = sprite.vertex_buf.as_ref().unwrap();
                    let i_buf = sprite.index_buf.as_ref().unwrap();

                    builder = builder.draw_indexed(
                        self.pipeline.clone(),
                        state.clone(),
                        vec![v_buf.clone()],
                        i_buf.clone(),
                        (instance_set.clone(), view_proj.clone(), tex_set.clone()),
                        (fs::ty::PER_OBJECT { imgIdx: sprite.image_index })
                    ).unwrap();
                },
                RenderId::TileStrip(e, idx) => {
                    let map = map.get(e).unwrap();
                    let strip = &map.strips[idx];

                    let instance_set = map.instance_set.as_ref().unwrap();
                    let v_buf = strip.vertex_buf.as_ref().unwrap();
                    let i_buf = strip.index_buf.as_ref().unwrap();

                    builder = builder.draw_indexed(
                        self.pipeline.clone(),
                        state.clone(),
                        vec![v_buf.clone()],
                        i_buf.clone(),
                        (instance_set.clone(), view_proj.clone(), tex_set.clone()),
                        (fs::ty::PER_OBJECT { imgIdx: map.image_index })
                    ).unwrap();
                }
            }
        }

        let command_buffer = 
            builder.end_render_pass().unwrap()
            .build().unwrap();

        // Sends the built command buffer off for execution.
        self.cmd_buf_tx.send(command_buffer)
            .expect("Couldn't send the command buffer, receiving end disconnected.");
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut tran_storage: specs::WriteStorage<comp::Transform> = SystemData::fetch(&res);
        self.transform_ins_read = Some(tran_storage.track_inserted());
        self.transform_mod_read = Some(tran_storage.track_modified());

        let mut sprite_storage: specs::WriteStorage<comp::Sprite> = SystemData::fetch(&res);
        self.sprite_ins_read = Some(sprite_storage.track_inserted());
        self.sprite_mod_read = Some(sprite_storage.track_modified());

        let mut tile_map_storage: specs::WriteStorage<comp::TileMap> = SystemData::fetch(&res);
        self.tile_map_ins_read = Some(tile_map_storage.track_inserted());
        self.tile_map_mod_read = Some(tile_map_storage.track_modified());
    }
}