pub mod types;

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
use shred;
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
        
#[derive(Clone)]
pub struct LuaWorld(pub *const specs::Resources);

unsafe impl Send for LuaWorld { }

pub type ScriptResult<T> = Result<T, ScriptError>;

macro_rules! script {
    (components: [ $(($lua_names:expr) = $comp_names:ident: $comp_types:ty),* ],
    // For types implementing LuaCtor
    types: [ $($types:ty),* ],
    functions: [ $(($func_names:expr) = $funcs:expr),* ]) => {
        impl UserData for LuaWorld {
            fn add_methods(methods: &mut UserDataMethods<Self>) {
                $(
                methods.add_method($func_names, $funcs);
                )*
            }
        }

        pub struct Script(());

        impl Script {
            pub fn new(lua: &Lua) -> Script {
                $(<$types as types::LuaCtor>::add_ctors(lua);)*

                Script(())
            }

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
        ("transform") = transform: comp::Transform,
        ("velocity")  = velocity: comp::Velocity,
        ("collider")  = collider: comp::Collider,
        ("sprite")    = sprite: comp::Sprite,
        ("tile_map")  = tile_map: comp::TileMap,
        ("script")    = script: comp::ScriptBehavior
    ],
    types: [
        types::Vector2f,
        types::Vector3f
    ],
    functions: [
        ("position") = |_, this: &LuaWorld, entity: LuaEntity| -> LuaResult<types::Vector3f> {
            unsafe {
                let res = &*this.0;
                let storage: specs::ReadStorage<comp::Transform> = specs::SystemData::fetch(&res);
                Ok(types::Vector3f(storage.get(entity.0).unwrap().pos))
            }
        },
        ("move") = |_, this: &LuaWorld, (entity, vec): (LuaEntity, types::Vector3f)| {
            unsafe {
                let res = &*this.0;
                let mut storage: specs::WriteStorage<comp::Transform> = specs::SystemData::fetch(&res);
                let comp = storage.get_mut(entity.0).unwrap();
                comp.pos += vec.0;
            }
            Ok(())
        },
        ("set_velocity") = |_, this: &LuaWorld, (entity, vec): (LuaEntity, types::Vector3f)| {
            unsafe {
                let res = &*this.0;
                let mut storage: specs::WriteStorage<comp::Velocity> = specs::SystemData::fetch(&res);
                let comp = storage.get_mut(entity.0).unwrap();
                comp.pos = vec.0;
            }
            Ok(())
        },
        ("is_pressed") = |_, this: &LuaWorld, input_index: usize| -> LuaResult<bool> {
            unsafe {
                let res = &*this.0;
                let input_list: shred::Fetch<res::input::InputList> = res.fetch();
                Ok(input_list.inputs[input_index].map(|x| x == res::input::InputState::Pressed).unwrap_or(false))
            }
        }
    ]
);