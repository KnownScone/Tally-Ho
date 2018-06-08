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
use specs;

// TODO: Implement view/projection matrix.
pub struct RenderSystem<L> {
    pipeline: Arc<L>,
    
    instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
    instance_buf: CpuBufferPool<vs::ty::Instance>,

    transform_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    transform_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    update_transform: specs::BitSet,

    render_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    render_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    update_render: specs::BitSet,

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
            update_transform: specs::BitSet::new(),
            render_ins_read: None,
            render_mod_read: None,
            update_render: specs::BitSet::new(),
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
        specs::WriteStorage<'a, comp::Render>,
        specs::ReadStorage<'a, comp::Transform>
    );

    fn run(&mut self, (device, queue, framebuffer, state, view_proj, tex_set, mesh_list, mut rndr, tran): Self::SystemData) {
        use specs::Join;

        let queue = queue.0.as_ref().unwrap();
        let device = device.0.as_ref().unwrap();
        let framebuffer = framebuffer.0.as_ref().unwrap();
        let state = state.0.as_ref().unwrap();
        let view_proj = view_proj.0.as_ref().unwrap();
        let tex_set = tex_set.0.as_ref().unwrap();
        let mesh_list = &mesh_list.0;

        // Get the components in need of initialization or an update
        self.update_render.clear();
        self.update_transform.clear();
        
        rndr.populate_inserted(&mut self.render_ins_read.as_mut().unwrap(), &mut self.update_render);
        rndr.populate_modified(&mut self.render_mod_read.as_mut().unwrap(), &mut self.update_render);
        tran.populate_inserted(&mut self.transform_ins_read.as_mut().unwrap(), &mut self.update_transform);
        tran.populate_modified(&mut self.transform_mod_read.as_mut().unwrap(), &mut self.update_transform);

        // Initializes newly-inserted render components' buffers and instance set.
        for (mut rndr, tran, _) in (&mut rndr, &tran, &self.update_transform).join() {
            let instance_data = vs::ty::Instance {
                transform: Matrix4::from_translation(Vector3::new(tran.x, tran.y, 0.0)).into(),
            };

            let instance_subbuf = self.instance_buf.next(instance_data)
                .expect("Couldn't build instance sub-buffer");

            // Creates a descriptor set with the newly-allocated subbuffer (containing our instance data).
            rndr.instance_set = Some(
                Arc::new(
                    self.instance_sets.next()
                        .add_buffer(instance_subbuf).unwrap()
                        .build().unwrap()
                )
            );
        }

        // TODO: In the future, check for transform modification and update the render.instance_set with it.

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

        // Adds a draw command on the command buffer for each render component.
        for rndr in (&rndr).join() {
            let mesh = &mesh_list[rndr.mesh_index];
            let instance_set = rndr.instance_set.as_ref().unwrap();

            builder = builder.draw_indexed(
                self.pipeline.clone(),
                state.clone(),
                vec![mesh.vertex_buf.clone()], 
                mesh.index_buf.clone(),
                (instance_set.clone(), view_proj.clone(), tex_set.clone()),
                (fs::ty::PER_OBJECT { imgIdx: rndr.image_index })
            ).unwrap();
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

        let mut rndr_storage: specs::WriteStorage<comp::Render> = SystemData::fetch(&res);
        self.render_ins_read = Some(rndr_storage.track_inserted());
        self.render_mod_read = Some(rndr_storage.track_modified());

        let mut tran_storage: specs::WriteStorage<comp::Transform> = SystemData::fetch(&res);
        self.transform_ins_read = Some(tran_storage.track_inserted());        
        self.transform_mod_read = Some(tran_storage.track_modified());        
    }
}