use ::{fs, vs};
use ::component as comp;
use ::resource as res;

use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use vulkano as vk;
use vk::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder};
use dmsort;
use specs;
use cgmath;

pub struct RenderSystem<L> {
    pipeline: Arc<L>,
    default_instance: Arc<vk::descriptor::DescriptorSet + Send + Sync>,
    
    cmd_buf_tx: mpsc::Sender<AutoCommandBuffer>
}

impl<L> RenderSystem<L> 
where
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    pub fn new(
        pipeline: Arc<L>,
        queue: Arc<vk::device::Queue>,
    ) -> (RenderSystem<L>, mpsc::Receiver<AutoCommandBuffer>) {
        use vk::descriptor::descriptor_set::*;
        use cgmath::One;

        let (tx, rx) = mpsc::channel();

        let (inst_buf, future) = vk::buffer::ImmutableBuffer::from_data(
            vs::ty::Instance {
                transform: cgmath::Matrix4::one().into(),
            },
            vk::buffer::BufferUsage::uniform_buffer(),
            queue
        ).unwrap();

        let set = PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(inst_buf).unwrap()
        .build().unwrap();

        (RenderSystem {
            pipeline: pipeline,
            default_instance: Arc::new(set),
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
        specs::Write<'a, res::SortedRender>,
        specs::ReadStorage<'a, comp::Sprite>,
        specs::ReadStorage<'a, comp::TileMap>,
        specs::ReadStorage<'a, comp::RenderStrip>,
        specs::ReadStorage<'a, comp::Transform>
    );

    fn run(&mut self, (device, queue, framebuffer, state, view_proj, tex_set, mut sort_rndr, sprite, map, strip, tran): Self::SystemData) {
        let queue = queue.0.as_ref().unwrap();
        let device = device.0.as_ref().unwrap();
        let framebuffer = framebuffer.0.as_ref().unwrap();
        let state = state.0.as_ref().unwrap();
        let view_proj = view_proj.0.as_ref().unwrap();
        let tex_set = tex_set.0.as_ref().unwrap();

        if sort_rndr.need_sort {
            dmsort::sort_by(&mut sort_rndr.ids, |id1, id2| {
                let get_values = |id: &res::RenderId| {
                    match *id {
                        res::RenderId::Sprite(e) => {
                            let t = tran.get(e).unwrap();
                            let s = sprite.get(e).unwrap();
                            let b = t.pos.y + s.bounds.max.y;

                            (t.pos.z, b)
                        },
                        res::RenderId::TileStrip(e) => {
                            let s = strip.get(e).unwrap();
                            let m = map.get(s.tile_map()).unwrap();
                            let b = m.tile_dims().y * (s.pos().y + 1) as f32;
                            let l = s.pos().z as f32 * m.tile_dims().z;

                            (l, b)
                        }
                    }
                };

                let (l1, b1) = get_values(id1);
                let (l2, b2) = get_values(id2);

                use std::cmp::Ordering;
                let order = l1.partial_cmp(&l2).unwrap();

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

        // Now to render.
        for id in sort_rndr.ids.iter() {
            match *id {
                res::RenderId::Sprite(e) => {
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
                res::RenderId::TileStrip(e) => {
                    let strip = strip.get(e).unwrap();
                    let map = map.get(strip.tile_map()).unwrap();

                    let v_buf = strip.vertex_buf.as_ref().unwrap();
                    let i_buf = strip.index_buf.as_ref().unwrap();

                    builder = builder.draw_indexed(
                        self.pipeline.clone(),
                        state.clone(),
                        vec![v_buf.clone()],
                        i_buf.clone(),
                        (self.default_instance.clone(), view_proj.clone(), tex_set.clone()),
                        (fs::ty::PER_OBJECT { imgIdx: map.image_index() })
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
    }
}