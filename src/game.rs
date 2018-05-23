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

// TODO: Try not to copy-cat functions from specs::World, at this point I might as well give public access to it
    pub fn create_entity(&mut self) -> specs::EntityBuilder {
        self.world.create_entity()
    }

    pub fn create_entity_unchecked(&self) -> specs::EntityBuilder {
        self.world.create_entity_unchecked()
    }

    pub fn delete_entity(&mut self, entity: specs::Entity) -> Result<(), specs::error::WrongGeneration> {
        self.world.delete_entity(entity)
    }

    pub fn read_storage<T: specs::Component>(&self) -> specs::ReadStorage<T> {
        self.world.read_storage::<T>()
    }

    pub fn tick(&mut self) {
        self.dispatcher.dispatch(&mut self.world.res);
    }
}

#[test]
fn create_game() {
    use system as sys;
    use component as comp;
    use script::Script;

    let velocity_sys = sys::VelocitySystem;

    let dispatcher = specs::DispatcherBuilder::new()
        .with(velocity_sys, "velocity", &[])
        .build();

    let mut game = Game::new(dispatcher);

    let mut script = Script::new();
    script.register::<comp::Transform>("transform");
    script.register::<comp::Velocity>("velocity");
    script.load_file("assets/scripts/test.lua");

    let e = script.parse_entity("stuff", game.create_entity()).unwrap();

    {
        let t_storage = game.read_storage::<comp::Transform>();
        let v_storage = game.read_storage::<comp::Velocity>();
        println!("BEFORE: {:?}, {:?}", t_storage.get(e).unwrap(), v_storage.get(e).unwrap());
    }

    game.tick();

    {
        let t_storage = game.read_storage::<comp::Transform>();
        let v_storage = game.read_storage::<comp::Velocity>();
        println!("AFTER: {:?}, {:?}", t_storage.get(e).unwrap(), v_storage.get(e).unwrap());
    }
}