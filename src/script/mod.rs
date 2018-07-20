mod parse;
pub use self::parse::ComponentParser;

use ::comp;

use std::sync::{Arc, Mutex};

use specs;
use cgmath::Vector3;
use rlua::{Lua, Table, Value as LuaValue, Result as LuaResult, Error as LuaError, String as LuaString, UserData, UserDataMethods};

#[derive(Debug)]
pub enum ScriptError {
    InvalidEntity(String),
    InvalidComponent(String),
    LuaError(LuaError),
}

impl From<LuaError> for ScriptError {
    fn from(error: LuaError) -> Self {
        ScriptError::LuaError(error)
    }
} 

pub type ScriptResult<T> = Result<T, ScriptError>;

macro_rules! script {
    ($($comp_names:ident: $comp_types:ty = $lua_names:expr),*) => {
        
        #[derive(Clone)]
        pub struct LuaEntity(pub specs::Entity);

        impl UserData for LuaEntity {
            fn add_methods(methods: &mut UserDataMethods<Self>) {
                methods.add_method("id", |_, this, _: ()| {
                    Ok(this.0.id())
                });
            }
        }

        pub struct Script;

        impl Script {
            fn parse_component<'a>(&self, lua: &Lua, lua_name: &str, data: LuaValue, eb: specs::EntityBuilder<'a>) -> ScriptResult<specs::EntityBuilder<'a>> {
                match lua_name {
                    $($lua_names => {
                        Ok(eb.with(
                            <$comp_types as ComponentParser>::parse(data, lua)?
                        ))
                    }),*
                    _ => Err(ScriptError::InvalidComponent(lua_name.into())),
                }
            }

            pub fn parse_entity(&self, lua: &Lua, lua_name: &str, mut eb: specs::EntityBuilder) -> ScriptResult<specs::Entity> {
                let globals = lua.globals();
                let ent_table: Table = globals.get(lua_name.clone())
                    .map_err(|x| ScriptError::InvalidEntity(lua_name.into()))?;

                for comp_pair in ent_table.pairs::<String, _>() {
                    let (comp_lua_name, comp_data) = comp_pair?;

                    eb = self.parse_component(lua, &comp_lua_name, comp_data, eb)?;
                }
                Ok(eb.build())
            }
        }
    };
}

script!(
    Transform: comp::Transform = "transform",
    Velocity: comp::Velocity = "velocity",
    Collider: comp::Collider = "collider",
    Sprite: comp::Sprite = "sprite",
    TileMap: comp::TileMap = "tile_map"
);