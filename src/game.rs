use system as sys;
use component as comp;

use std::fs::File;
use std::io::prelude::*;

use specs;
use rlua::{Lua, Table, Function, MetaMethod, Result as LuaResult, UserData, UserDataMethods, Variadic};

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
    
    let lua = Lua::new();

    let globals = lua.globals();

    let mut file = File::open("assets/scripts/test.lua").expect("File was not found");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Couldn't read the file");
   
    let init = lua.load(&contents, Some("test.lua")).unwrap();
    init.call::<_, ()>(());

    let g: String = globals.get("g").unwrap();

    {
        // TODO: Generalize this code into some structs/functions

        let entity: Table = globals.get("entity").unwrap();
        let mut e1 = game.create_entity();

        for pair in entity.pairs::<String, Table>() {
            let (comp_type, comp_data) = pair.unwrap();
            e1 = match comp_type.as_ref() {
                "transform" =>
                    e1.with(comp::Transform { 
                        x: comp_data.get("x").expect("Uh"), 
                        y: comp_data.get("y").expect("Uh")
                    }),
                "velocity" =>
                    e1.with(comp::Velocity { 
                        x: comp_data.get("x").expect("Uh"), 
                        y: comp_data.get("y").expect("Uh")
                    }),
                _ => continue
            }
        }

        let e1 = e1.build();
    }

    game.tick();
}