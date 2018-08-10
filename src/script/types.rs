use cgmath;
use rlua::{UserData, UserDataMethods, MetaMethod, Lua, Result as LuaResult};

pub trait LuaCtor {
    fn add_ctors(lua: &Lua);
}

#[derive(Clone)]
pub struct Vector2f(pub cgmath::Vector2<f32>);

impl UserData for Vector2f {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        use cgmath::InnerSpace;

        methods.add_method_mut("normalize", |_, this, ()| -> LuaResult<Self> {
            let mut ret = this.clone();
            ret.0 = this.0.normalize();

            Ok(ret)
        });

        methods.add_meta_method_mut(MetaMethod::Add, |_, this, vec: Vector2f| -> LuaResult<Self> {
            Ok(Vector2f(this.0 + vec.0))
        });
    }
}

impl LuaCtor for Vector2f {
    fn add_ctors(lua: &Lua) {
        lua.globals().set(
            "vec2f", 
            lua.create_function(|_, (x, y)| 
                Ok(Vector2f(cgmath::Vector2::new(x, y)))
            ).unwrap()
        );
    }
}

#[derive(Clone)]
pub struct Vector3f(pub cgmath::Vector3<f32>);

impl UserData for Vector3f {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        use cgmath::InnerSpace;

        methods.add_method_mut("normalized", |_, this, ()| -> LuaResult<Self> {
            let mut ret = this.clone();
            ret.0 = this.0.normalize();

            Ok(ret)
        });

        methods.add_meta_method_mut(MetaMethod::Add, |_, this, vec: Vector3f| -> LuaResult<Self> {
            Ok(Vector3f(this.0 + vec.0))
        });
    }
}

impl LuaCtor for Vector3f {
    fn add_ctors(lua: &Lua) {
        lua.globals().set(
            "vec3f", 
            lua.create_function(|_, (x, y, z)| 
                Ok(Vector3f(cgmath::Vector3::new(x, y, z)))
            ).unwrap()
        );
    }
}