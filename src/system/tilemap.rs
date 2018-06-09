use ::component as comp;
use ::resource as res;

use specs;

pub struct TileMapSystem {
    tile_map_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    tile_map_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,

    update_tile_map: specs::BitSet,
    
    transform_ins_read: Option<specs::ReaderId<specs::InsertedFlag>>,
    transform_mod_read: Option<specs::ReaderId<specs::ModifiedFlag>>,
    
    update_transform: specs::BitSet,
}

impl<'a> specs::System<'a> for TileMapSystem {
    type SystemData = (
        specs::ReadStorage<'a, comp::Transform>, 
        specs::WriteStorage<'a, comp::TileMap>,
    );

    fn run(&mut self, (tran, mut map): Self::SystemData) {
        use specs::Join;

        // Get the components in need of initialization or an update
        self.update_tile_map.clear();
        self.update_transform.clear();
        
        map.populate_inserted(&mut self.map_ins_read.as_mut().unwrap(), &mut self.update_tile_map);
        map.populate_modified(&mut self.map_mod_read.as_mut().unwrap(), &mut self.update_tile_map);
        tran.populate_inserted(&mut self.transform_ins_read.as_mut().unwrap(), &mut self.update_transform);
        tran.populate_modified(&mut self.transform_mod_read.as_mut().unwrap(), &mut self.update_transform);

        for (tran, mut map, _) in (&tran, &mut map, self.update_transform).join() {
            
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