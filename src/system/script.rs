use ::component as comp;
use ::resource as res;
use script::{LuaEntity, LuaWorld};

use rlua::{Function as LuaFunction};
use cgmath::{Zero, Vector3};
use specs;

pub struct OnTickEvent;

impl<'a> specs::RunNow<'a> for OnTickEvent {
    fn run_now(&mut self, res: &'a specs::Resources) {
        use specs::Join;

        let (ent, script, lua, dt): (specs::Entities, specs::ReadStorage<comp::ScriptBehavior>, specs::Read<res::Lua>, specs::Read<res::DeltaTime>) = specs::SystemData::fetch(&res);

        let dt = dt.0;
        if let Some(ref mutex) = lua.0 {
            let lua = mutex.lock().unwrap();

            for (ent, script) in (&*ent, &script).join() {
                unsafe {
                    let res = res as *const _;

                    let on_tick = script.on_tick.as_ref();

                    if let Some(func) = on_tick.and_then(|x| lua.registry_value::<LuaFunction>(&x).ok()) {
                        func.call::<_, ()>((LuaWorld(res), LuaEntity(ent), dt)).unwrap();
                    }
                }
            }
        }
    }
    
    fn setup(&mut self, res: &mut specs::Resources) {
        <specs::ReadStorage<comp::ScriptBehavior> as specs::SystemData>::setup(res);
    }
}