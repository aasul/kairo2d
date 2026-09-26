use mlua::{Lua, Table};

pub(crate) fn register(lua: &Lua) -> mlua::Result<()> {
    let ui: Table = lua
        .load(include_str!("builtin/ui.lua"))
        .set_name("@kairo/ui.lua")
        .eval()?;
    lua.globals().set("ui", ui)
}

pub(crate) fn call<A: mlua::IntoLuaMulti>(lua: &Lua, name: &str, args: A) -> mlua::Result<()> {
    let ui: Table = lua.globals().get("ui")?;
    ui.get::<mlua::Function>(name)?.call(args)
}
