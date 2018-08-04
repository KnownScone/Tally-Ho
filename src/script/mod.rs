mod parse;
pub use self::parse::ComponentParser;

use ::resource as res;
use ::component as comp;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::cell::{RefCell};
use std::rc::Rc;

use specs;
use specs::Builder;
use shred::{Accessor, AccessorCow, CastFrom, DispatcherBuilder, DynamicSystemData, MetaTable, Read, Resource,
            ResourceId, Resources,
            System, SystemData};
use shred::cell::{Ref, RefMut};
use cgmath::Vector3;
use rlua::{Lua, Table, RegistryKey, Value as LuaValue, Result as LuaResult, Function as LuaFunction, Error as LuaError, String as LuaString,
    UserData, UserDataMethods, AnyUserData, Scope as LuaScope};

#[derive(Clone)]
pub struct LuaEntity(pub specs::Entity);

impl UserData for LuaEntity {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        methods.add_method("id", |_, this, _: ()| {
            Ok(this.0.id())
        });
    }
}

#[derive(Clone)]
pub struct LuaWorld(pub *mut specs::World);

unsafe impl Send for LuaWorld { }

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
    (components: [ $(($lua_names:expr) = $comp_names:ident: $comp_types:ty),* ],
    functions: [ $(($func_names:expr) = $funcs:expr),* ]) => {

        impl UserData for LuaWorld {
            fn add_methods(methods: &mut UserDataMethods<Self>) {
                $(
                methods.add_method($func_names, $funcs);
                )*
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
    components: [
        ("transform") = Transform: comp::Transform,
        ("velocity")  = Velocity: comp::Velocity,
        ("collider")  = Collider: comp::Collider,
        ("sprite")    = Sprite: comp::Sprite,
        ("tile_map")  = TileMap: comp::TileMap
    ],
    functions: [
        ("position") = |_, this: &LuaWorld, entity: LuaEntity| -> LuaResult<(f32, f32, f32)> {
            unsafe {
                let world = &*this.0;
                let storage = world.read_storage::<comp::Transform>();
                Ok(storage.get(entity.0).unwrap().pos.into())
            }
        },
        ("move") = |_, this: &LuaWorld, (entity, x, y, z): (LuaEntity, f32, f32, f32)| {
            unsafe {
                let world = &*this.0;
                let mut storage = world.write_storage::<comp::Transform>();
                let comp = storage.get_mut(entity.0).unwrap();
                comp.pos += Vector3::new(x, y, z);
            }
            Ok(())
        }
    ]
);