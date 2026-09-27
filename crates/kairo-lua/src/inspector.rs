use crate::state::{lua_error, SharedState};
use anyhow::Result;
use kairo_core::inspector::{
    InspectMetadata, InspectNode, InspectSnapshot, InspectUpdate, SceneStatus,
};
use mlua::{Lua, LuaSerdeExt, Table, Value};

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let backend = lua.create_table()?;
    backend.set(
        "describe",
        lua.create_function(|lua, (path, value, metadata): (String, Value, Table)| {
            let metadata: InspectMetadata = lua.from_value(Value::Table(metadata))?;
            let value = lua.from_value(value)?;
            let node = InspectNode {
                path,
                value,
                metadata,
            };
            node.validate().map_err(lua_error)?;
            lua.to_value(&node)
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "validateUpdate",
        lua.create_function(move |lua, (node, update): (Value, Value)| {
            let node: InspectNode = lua.from_value(node)?;
            let update: InspectUpdate = lua.from_value(update)?;
            node.validate_update(&update, s.borrow().inspection_session)
                .map_err(lua_error)
        })?,
    )?;
    let public: Table = lua
        .load(include_str!("builtin/inspector.lua"))
        .set_name("@kairo/inspector.lua")
        .call(backend)?;
    lua.globals().set("inspector", public)
}
pub(crate) fn snapshot(lua: &Lua, session: u64) -> Result<InspectSnapshot> {
    let inspector: Table = lua.globals().get("inspector")?;
    let value: Value = inspector.get::<mlua::Function>("_snapshot")?.call(())?;
    let nodes = lua.from_value(value)?;
    let scene: Table = lua.globals().get("scene")?;
    let value: Value = scene.get::<mlua::Function>("info")?.call(())?;
    let scenes: SceneStatus = lua.from_value(value)?;
    let result = InspectSnapshot {
        session,
        nodes,
        scenes,
        error: None,
    };
    result.validate()?;
    Ok(result)
}
pub(crate) fn apply(lua: &Lua, update: InspectUpdate) -> Result<()> {
    let inspector: Table = lua.globals().get("inspector")?;
    inspector
        .get::<mlua::Function>("_apply")?
        .call::<()>(lua.to_value(&update)?)?;
    Ok(())
}
