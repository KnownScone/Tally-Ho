use ::{vs, Vertex};
use ::component as comp;
use ::resource as res;

use std::sync::Arc;

use vulkano as vk;
use vk::descriptor::descriptor_set::FixedSizeDescriptorSetsPool;
use vk::buffer::CpuBufferPool;
use cgmath::{Vector3, Matrix4};
use specs;

pub struct TileMapSystem<L> {
    instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
    instance_buf: CpuBufferPool<vs::ty::Instance>,

    tile_map_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    tile_map_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    ins_tile_map: specs::BitSet,
    mod_tile_map: specs::BitSet,
    
    transform_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    transform_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    updt_transform: specs::BitSet,
}

impl<L> TileMapSystem<L>
where
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    pub fn new(
        instance_sets: FixedSizeDescriptorSetsPool<Arc<L>>,
        instance_buf: CpuBufferPool<vs::ty::Instance>,
    ) -> TileMapSystem<L> {
        TileMapSystem {
            instance_sets,
            instance_buf,
            tile_map_ins_read: None,
            tile_map_mod_read: None,
            ins_tile_map: specs::BitSet::new(),
            mod_tile_map: specs::BitSet::new(),
            transform_ins_read: None,
            transform_mod_read: None,
            updt_transform: specs::BitSet::new(),
        }
    }
}

impl<'a, L> specs::System<'a> for TileMapSystem<L>
where
    L: vk::descriptor::PipelineLayoutAbstract + vk::pipeline::GraphicsPipelineAbstract + Send + Sync + 'static,
{
    type SystemData = (
        specs::Read<'a, res::Queue>,
        specs::Write<'a, res::SortedRender>,
        specs::Entities<'a>, 
        specs::ReadStorage<'a, comp::Transform>, 
        specs::WriteStorage<'a, comp::TileMap>,
    );

    fn run(&mut self, (queue, mut sort_rndr, ent, tran, mut map): Self::SystemData) {
        use specs::Join;

        let queue = queue.0.as_ref().unwrap();

        // Get the components in need of initialization or an update.
        self.ins_tile_map.clear();
        self.mod_tile_map.clear();
        self.updt_transform.clear();
        
        map.populate_inserted(&mut self.tile_map_ins_read.as_mut().unwrap(), &mut self.ins_tile_map);
        map.populate_modified(&mut self.tile_map_mod_read.as_mut().unwrap(), &mut self.mod_tile_map);
        tran.populate_inserted(&mut self.transform_ins_read.as_mut().unwrap(), &mut self.updt_transform);
        tran.populate_modified(&mut self.transform_mod_read.as_mut().unwrap(), &mut self.updt_transform);

        for (ent, mut map, _) in (&*ent, &mut map, &self.ins_tile_map | &self.mod_tile_map).join() {
            let dims = map.tile_dims;

            // Create the vertex and index buffers for all the strips without them.
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

                // If this tile map strip has just being initialized, add it to the sorted renders.
                if !strip.is_init {
                    sort_rndr.ids.push(res::RenderId::TileStrip(ent, idx));
                    strip.is_init = true;
                }

                // After updating the tile map strip's data, the tile map strip needs to be resorted.
                sort_rndr.need_sort = true;
            }
        }

        for (mut map, tran, _) in (&mut map, &tran, &self.updt_transform).join() {
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

            map.instance_set = Some(set);

            // After updating the transform data, the tile map needs to be resorted.
            sort_rndr.need_sort = true;
        }
    }

    fn setup(&mut self, res: &mut specs::Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);

        let mut map_storage: specs::WriteStorage<comp::TileMap> = SystemData::fetch(&res);
        self.tile_map_ins_read = Some(map_storage.track_inserted());
        self.tile_map_mod_read = Some(map_storage.track_modified());

        let mut tran_storage: specs::WriteStorage<comp::Transform> = SystemData::fetch(&res);
        self.transform_ins_read = Some(tran_storage.track_inserted());        
        self.transform_mod_read = Some(tran_storage.track_modified());        
    }
}