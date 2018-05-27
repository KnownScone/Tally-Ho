use ::Vertex;
use ::vs;
use ::component as comp;
use ::resource as res;

use std::sync::Arc;
use std::sync::mpsc;

use vulkano as vk;
use vk::buffer::{CpuBufferPool, DeviceLocalBuffer, BufferSlice};
use vk::descriptor::PipelineLayoutAbstract;
use vk::descriptor::descriptor_set::{FixedSizeDescriptorSet, FixedSizeDescriptorSetsPool};
use vk::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder};
use cgmath::{Matrix4, Vector3};
use specs;

// TODO: Implement view/projection matrix
pub struct RenderSystem<L> {
    pipeline: Arc<L>,
    
    instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
    instance_buf: CpuBufferPool<vs::ty::Instance>,

    inserted_id: Option<specs::ReaderId<specs::InsertedFlag>>,
    inserted: specs::BitSet,

    cmd_buf_tx: mpsc::Sender<AutoCommandBuffer>
}

impl<L> RenderSystem<L> 
where
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    pub fn new(
        pipeline: Arc<L>,
        vertex_buf: (Arc<DeviceLocalBuffer<[Vertex]>>, CpuBufferPool<Vertex>), 
        index_buf: (Arc<DeviceLocalBuffer<[u32]>>, CpuBufferPool<u32>),
        instance_buf: CpuBufferPool<vs::ty::Instance>,
    ) -> (RenderSystem<L>, mpsc::Receiver<AutoCommandBuffer>) {
        let (tx, rx) = mpsc::channel();

        (RenderSystem {
            pipeline: pipeline.clone(),
            instance_sets: FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0),
            instance_buf,
            inserted_id: None,
            inserted: specs::BitSet::new(),
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
        specs::WriteStorage<'a, comp::StaticRender>
    );

    fn run(&mut self, (device, queue, framebuffer, state, mut rndr): Self::SystemData) {
        use specs::Join;

        let queue = queue.0.as_ref().unwrap();
        let device = device.0.as_ref().unwrap();
        let framebuffer = framebuffer.0.as_ref().unwrap();
        let state = state.0.as_ref().unwrap();

        // Get the components in need of initialization
        self.inserted.clear();
        rndr.populate_inserted(&mut self.inserted_id.as_mut().unwrap(), &mut self.inserted);

        // Initialize buffers / instance set
        for (rndr, _) in (&mut rndr, &self.inserted).join() {
            let (index_buf, future) = vk::buffer::ImmutableBuffer::from_iter(
                rndr.index_data.iter().cloned(),
                vk::buffer::BufferUsage::index_buffer(),
                queue.clone()
            ).expect("Couldn't create index buffer");

            rndr.index_buf = Some(index_buf);

            let (vertex_buf, future) = vk::buffer::ImmutableBuffer::from_iter(
                rndr.vertex_data.iter().cloned(),
                vk::buffer::BufferUsage::vertex_buffer(),
                queue.clone()
            ).expect("Couldn't create vertex buffer");

            rndr.vertex_buf = Some(vertex_buf);

            let instance_data = vs::ty::Instance {
                transform: Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0)).into(),
            };

            let instance_subbuf = self.instance_buf.next(instance_data)
                .expect("Couldn't build instance sub-buffer");

            rndr.instance_set = Some(
                Arc::new(
                    self.instance_sets.next()
                        .add_buffer(instance_subbuf).unwrap()
                        .build().unwrap()
                )
            );
        }

        // TODO: In the future, check for transform modification and update the render.instance_set with it

        let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
            .begin_render_pass(
                framebuffer.clone(), 
                false, vec![[0.0, 0.0, 1.0, 1.0].into()]
            ).unwrap();

        for rndr in (&rndr).join() {
            builder = builder.draw_indexed(
                self.pipeline.clone(),
                state.clone(),
                vec![rndr.vertex_buf.as_ref().unwrap().clone()], 
                rndr.index_buf.as_ref().unwrap().clone(),
                (rndr.instance_set.as_ref().unwrap().clone()),
                ()
            ).unwrap();
        }

        let command_buffer = 
            builder.end_render_pass().unwrap()
            
            .build().unwrap();

        self.cmd_buf_tx.send(command_buffer);
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut rndr_storage: specs::WriteStorage<comp::StaticRender> = SystemData::fetch(&res);
        self.inserted_id = Some(rndr_storage.track_inserted());
    }
}