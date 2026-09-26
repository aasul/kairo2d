use crate::state::{with_state, SharedState};
use kairo_core::actions::{ActionBinding, AxisBinding, InputBindings};
use mlua::{Lua, LuaSerdeExt, Table};

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set(
        "bind",
        lua.create_function(move |lua, (name, value): (String, Table)| {
            let binding: ActionBinding = lua.from_value(mlua::Value::Table(value))?;
            with_state(&s, |state| {
                let mut bindings = state.actions.bindings.clone();
                bindings.actions.insert(name, binding);
                state.actions.replace(bindings)
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "bindAxis",
        lua.create_function(move |lua, (name, value): (String, Table)| {
            let binding: AxisBinding = lua.from_value(mlua::Value::Table(value))?;
            with_state(&s, |state| {
                let mut bindings = state.actions.bindings.clone();
                bindings.axes.insert(name, binding);
                state.actions.replace(bindings)
            })
        })?,
    )?;
    for (name, field) in [("action", 0), ("held", 0), ("pressed", 1), ("released", 2)] {
        let s = state.clone();
        module.set(
            name,
            lua.create_function(move |_, name: String| {
                with_state(&s, |state| {
                    let value = state.actions.action(&name)?;
                    Ok(match field {
                        1 => value.pressed,
                        2 => value.released,
                        _ => value.held,
                    })
                })
            })?,
        )?;
    }
    let s = state.clone();
    module.set(
        "axis",
        lua.create_function(move |_, name: String| {
            with_state(&s, |state| state.actions.axis(&name))
        })?,
    )?;
    let s = state.clone();
    module.set(
        "load",
        lua.create_function(move |_, path: Option<String>| {
            with_state(&s, |state| {
                let bindings = InputBindings::parse(
                    &state
                        .fs
                        .read_text(path.as_deref().unwrap_or("input.toml"))?,
                )?;
                state.actions.replace(bindings)
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "serialize",
        lua.create_function(move |_, ()| {
            with_state(&s, |state| {
                Ok(toml::to_string_pretty(&state.actions.bindings)?)
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "apply",
        lua.create_function(move |_, text: String| {
            with_state(&s, |state| {
                state.actions.replace(InputBindings::parse(&text)?)
            })
        })?,
    )?;
    lua.globals().set("input", module)
}
