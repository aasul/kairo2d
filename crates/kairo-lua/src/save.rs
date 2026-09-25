use crate::state::{with_state, SharedState};
use kairo_project::saves::SaveStore;
use mlua::{Lua, LuaSerdeExt, Value};

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set("write", lua.create_function(move |lua, (name, value): (String, Value)| {
        let value: serde_json::Value = lua.from_value(value)?;
        with_state(&s, |state| {
            anyhow::ensure!(!state.loading, "save.write is not allowed during game.load; write from game.update or game.quit");
            let store = SaveStore::open(&state.save_identity)?;
            store.write(&name, &value)
        })
    })?)?;
    let s = state.clone();
    module.set(
        "read",
        lua.create_function(move |lua, name: String| {
            let value = with_state(&s, |state| {
                SaveStore::open(&state.save_identity)?.read(&name)
            })?;
            match value {
                Some(value) => lua.to_value(&value),
                None => Ok(Value::Nil),
            }
        })?,
    )?;
    let s = state.clone();
    module.set(
        "exists",
        lua.create_function(move |_, name: String| {
            with_state(&s, |state| {
                SaveStore::open(&state.save_identity)?.exists(&name)
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "remove",
        lua.create_function(move |_, name: String| {
            with_state(&s, |state| {
                anyhow::ensure!(
                    !state.loading,
                    "save.remove is not allowed during game.load"
                );
                SaveStore::open(&state.save_identity)?.remove(&name)
            })
        })?,
    )?;
    let module: mlua::Table = lua
        .load(include_str!("builtin/save_migrations.lua"))
        .set_name("@kairo/save_migrations.lua")
        .call(module)?;
    lua.globals().set("save", module)
}
