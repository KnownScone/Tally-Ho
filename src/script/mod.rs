mod parse;
pub use self::parse::ComponentParser;

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;

use specs;
use rlua::{Lua, Table, Value as LuaValue, Result as LuaResult, Error as LuaError};

// A 'CompCtor' simply parses a component from lua and adds it onto an EntityBuilder
type CompCtor = for<'a> Fn(LuaValue, specs::EntityBuilder<'a>) -> specs::EntityBuilder<'a>;

pub struct Script {
    lua: Lua,
    comp_ctor: HashMap<String, Arc<CompCtor>>
}

impl Script {
    pub fn new() -> Script {
        Script {
            lua: Lua::new(),
            comp_ctor: HashMap::new()
        }
    }

    pub fn register<T: specs::Component + ComponentParser>(&mut self, alias: &str) {
        self.comp_ctor.insert(
            String::from(alias), 
            Arc::new(
                |v: LuaValue, eb: specs::EntityBuilder|
                    eb.with(
                        T::parse(v)
                            .expect("Couldn't parse component")
                    )
            )
        );
    }

    pub fn load_file(&mut self, path: &str) {
        let mut file = File::open(path)
            .expect("File was not found");
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Couldn't read the file");
        
        self.lua.exec::<()>(&contents, Some("test.lua"))
            .expect("Script failed to execute");
    }

    pub fn parse_entity(&self, name: &str, mut eb: specs::EntityBuilder) -> LuaResult<specs::Entity> {
        let globals = self.lua.globals();

        let ent_table: Table = globals.get(name.clone())
            .map_err(|x| LuaError::SyntaxError {
                message: format!("Entity with name \"{}\" does not exist", name),
                incomplete_input: false,
            })?;

        // TODO: In the case that we have two components of the same type on the same entity, return an error
        for comp_pair in ent_table.pairs::<String, _>() {
            let (comp_alias, comp_data) = comp_pair?;

            let ctor = self.comp_ctor.get(&comp_alias)
                .ok_or(LuaError::SyntaxError {
                    message: format!("{} is not a registered component", comp_alias),
                    incomplete_input: false,
                })?;

            eb = ctor(comp_data, eb);
        }

        Ok(eb.build())
    }
}