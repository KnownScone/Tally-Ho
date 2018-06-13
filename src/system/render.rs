use ::{vs, fs};
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

// TODO: Implement view/projection matrix.
pub struct RenderSystem<L> {
    pipeline: Arc<L>,
    
    instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
    instance_buf: CpuBufferPool<vs::ty::Instance>,

    transform_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    transform_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    inserted_transform: specs::BitSet,
    modified_transform: specs::BitSet,

    sorted: Vec<specs::Entity>,

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
        specs::Read<'a, res::MeshList>,
        specs::Entities<'a>,
        specs::WriteStorage<'a, comp::Sprite>,
        specs::WriteStorage<'a, comp::TileMap>,
        specs::ReadStorage<'a, comp::Transform>
    );

    fn run(&mut self, (device, queue, framebuffer, state, view_proj, tex_set, mesh_list, ent, mut sprite, mut map, tran): Self::SystemData) {
        use specs::Join;

        let queue = queue.0.as_ref().unwrap();
        let device = device.0.as_ref().unwrap();
        let framebuffer = framebuffer.0.as_ref().unwrap();
        let state = state.0.as_ref().unwrap();
        let view_proj = view_proj.0.as_ref().unwrap();
        let tex_set = tex_set.0.as_ref().unwrap();
        let mesh_list = &mesh_list.0;

        // Get the components in need of initialization or an update
        self.inserted_transform.clear();
        self.modified_transform.clear();
        
        tran.populate_inserted(&mut self.transform_ins_read.as_mut().unwrap(), &mut self.inserted_transform);
        tran.populate_modified(&mut self.transform_mod_read.as_mut().unwrap(), &mut self.modified_transform);

        let mut need_sort = false;

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

            if self.inserted_transform.contains(ent.id()) {
                self.sorted.push(ent);
            }

            need_sort = true;
        }

        if need_sort {
            dmsort::sort_by(&mut self.sorted, |e1, e2| {
                let t1 = tran.get(*e1).unwrap();
                let t2 = tran.get(*e2).unwrap();

                use std::cmp::Ordering;
                let order = t1.pos.z.partial_cmp(&t2.pos.z).unwrap();
                
                match order {
                    Ordering::Less => order,
                    Ordering::Greater => order,
                    Ordering::Equal => {
                        let b1 = t1.pos.y + t1.bounds.max.y;
                        let b2 = t2.pos.y + t2.bounds.max.y;
                        
                        if e1.id() == 1 || e2.id() == 1 {
                            info!("{} - {} - {:?}", b1, b2, b1.partial_cmp(&b2).unwrap());
                        }

                        b1.partial_cmp(&b2).unwrap()
                    }
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

        for ent in self.sorted.iter() {
            if let Some(sprite) = sprite.get_mut(*ent) {
                let mesh = &mesh_list[sprite.mesh_index];
                let instance_set = sprite.instance_set.as_ref().unwrap();

                builder = builder.draw_indexed(
                    self.pipeline.clone(),
                    state.clone(),
                    vec![mesh.vertex_buf.clone()], 
                    mesh.index_buf.clone(),
                    (instance_set.clone(), view_proj.clone(), tex_set.clone()),
                    (fs::ty::PER_OBJECT { imgIdx: sprite.image_index })
                ).unwrap();
            } else if let Some(map) = map.get_mut(*ent) {
                let instance_set = map.instance_set.as_ref().unwrap();

                for chunk in &map.chunks {
                    builder = builder.draw_indexed(
                        self.pipeline.clone(),
                        state.clone(),
                        vec![chunk.vertex_buf.clone()], 
                        chunk.index_buf.clone(),
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
    }
}