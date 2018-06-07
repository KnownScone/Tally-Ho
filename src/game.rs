use resource as res;

use std::time::{Instant, Duration};

use specs;

pub struct Scene {

}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            
        }
    }
}

pub struct Game<'a> {
    // TODO: Resource management

    // Graphical aspect - tilemaps, sprites, vfx, etc.
    scene: Scene,
    
    // TODO: Systems - logic
    dispatcher: specs::Dispatcher<'static, 'a>,

    last_update: Option<Instant>,
    
    pub world: specs::World,
}

impl<'a> Game<'a> {
    pub fn new(mut dispatcher: specs::Dispatcher<'static, 'a>) -> Game<'a> {
        let mut world = specs::World::new();
        
        world.add_resource(res::DeltaTime(0.0));   
        
        // Register the components and resources used in the registered systems (with default values)
        dispatcher.setup(&mut world.res);

        Game {
            scene: Scene::new(),
            dispatcher,
            last_update: None,
            world,
        }
    }

    pub fn update(&mut self, time_scale: f32) {
        if let Some(lu) = self.last_update {
            let dur = lu.elapsed();
            let dt = dur.as_secs() as f32 + dur.subsec_nanos() as f32 / 1_000_000_000.0;

            (*self.world.write_resource::<res::DeltaTime>()).0 = dt * time_scale;
        }

        self.dispatcher.dispatch(&mut self.world.res);

        self.last_update = Some(Instant::now());
    }
}