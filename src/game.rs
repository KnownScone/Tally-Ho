use system as sys;
use component as comp;

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
    
    world: specs::World,
}

impl<'a> Game<'a> {
    pub fn new(mut dispatcher: specs::Dispatcher<'static, 'a>) -> Game<'a> {
        let mut world = specs::World::new();
        
        // Register the components and resources used in the registered systems (with default values)
        dispatcher.setup(&mut world.res);

        Game {
            scene: Scene::new(),
            dispatcher,
            world,
        }
    }

    pub fn create_entity(&mut self) -> specs::EntityBuilder {
        self.world.create_entity()
    }

    pub fn delete_entity(&mut self, entity: specs::Entity) -> Result<(), specs::error::WrongGeneration> {
        self.world.delete_entity(entity)
    }

    pub fn tick(&mut self) {
        self.dispatcher.dispatch(&mut self.world.res);
    }
}

#[test]
fn create_game() {
    let velocity_sys = sys::VelocitySystem;

    let dispatcher = specs::DispatcherBuilder::new()
        .with(velocity_sys, "velocity", &[])
        .build();

    let mut game = Game::new(dispatcher);
    let e1 = game.create_entity()
        .with(comp::Transform { x: 0., y: 0. })
        .with(comp::Velocity { x: 0., y: 0. })
    .build();

    game.tick();

    game.delete_entity(e1);
}