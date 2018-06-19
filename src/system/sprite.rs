use ::vs;
use ::Vertex;
use ::component as comp;
use ::resource as res;

use std::sync::Arc;

use vulkano as vk;
use vk::descriptor::descriptor_set::FixedSizeDescriptorSetsPool;
use vk::buffer::CpuBufferPool;
use cgmath::{Vector3, Matrix4};
use specs;

pub struct SpriteSystem<L> {
    instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
    instance_buf: CpuBufferPool<vs::ty::Instance>,

    sprite_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    sprite_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    ins_sprite: specs::BitSet,
    mod_sprite: specs::BitSet,
    
    transform_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    transform_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    updt_transform: specs::BitSet,
}


impl<L> SpriteSystem<L>
where
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    pub fn new(
        instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
        instance_buf: CpuBufferPool<vs::ty::Instance>,
    ) -> SpriteSystem<L> {
        SpriteSystem {
            instance_sets,
            instance_buf,
            sprite_ins_read: None,
            sprite_mod_read: None,
            ins_sprite: specs::BitSet::new(),
            mod_sprite: specs::BitSet::new(),
            transform_ins_read: None,
            transform_mod_read: None,
            updt_transform: specs::BitSet::new(),
        }
    }
}

impl<'a, L> specs::System<'a> for SpriteSystem<L>
where
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    type SystemData = (
        specs::Read<'a, res::Queue>,
        specs::Write<'a, res::SortedRender>,
        specs::Entities<'a>, 
        specs::ReadStorage<'a, comp::Transform>, 
        specs::WriteStorage<'a, comp::Sprite>,
    );

    fn run(&mut self, (queue, mut sort_rndr, ent, tran, mut spr): Self::SystemData) {
        use specs::Join;

        let queue = queue.0.as_ref().unwrap();

        // Get the components in need of initialization or an update.
        self.ins_sprite.clear();
        self.mod_sprite.clear();
        self.updt_transform.clear();
        
        spr.populate_inserted(&mut self.sprite_ins_read.as_mut().unwrap(), &mut self.ins_sprite);
        spr.populate_modified(&mut self.sprite_mod_read.as_mut().unwrap(), &mut self.mod_sprite);
        tran.populate_inserted(&mut self.transform_ins_read.as_mut().unwrap(), &mut self.updt_transform);
        tran.populate_modified(&mut self.transform_mod_read.as_mut().unwrap(), &mut self.updt_transform);

        for (ent, mut spr, _) in (&*ent, &mut spr, &self.ins_sprite | &self.mod_sprite).join() {
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

            if self.ins_sprite.contains(ent.id()) {
                sort_rndr.ids.push(res::RenderId::Sprite(ent));
            }

            sort_rndr.need_sort = true;
        }

        for (mut spr, tran, _) in (&mut spr, &tran, &self.updt_transform).join() {
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

            spr.instance_set = Some(set);

            sort_rndr.need_sort = true;
        }
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut spr_storage: specs::WriteStorage<comp::Sprite> = SystemData::fetch(&res);
        self.sprite_ins_read = Some(spr_storage.track_inserted());
        self.sprite_mod_read = Some(spr_storage.track_modified());

        let mut tran_storage: specs::WriteStorage<comp::Transform> = SystemData::fetch(&res);
        self.transform_ins_read = Some(tran_storage.track_inserted());        
        self.transform_mod_read = Some(tran_storage.track_modified());        
    }
}