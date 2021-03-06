use rlua::{Value as LuaValue, Result as LuaResult, Lua};
use specs;
use script::ScriptResult;

// TODO: Create a macro to set up a component parser. Something in which you list the names of the fields you want 
// TODO: parsed, such as: component_parser!(ComponentStruct, "x", "y", "z", "other_data")
pub trait ComponentParser: Sized + specs::Component {
    fn parse(LuaValue, &Lua) -> ScriptResult<Self>;
}