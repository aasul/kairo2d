use crate::state::{with_state, SharedState};
use mlua::Lua;

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set(
        "connected",
        lua.create_function(move |lua, ()| {
            let ids = with_state(&s, |state| {
                Ok(state
                    .input
                    .gamepads
                    .connected
                    .keys()
                    .copied()
                    .collect::<Vec<_>>())
            })?;
            lua.create_sequence_from(ids)
        })?,
    )?;
    let s = state.clone();
    module.set(
        "name",
        lua.create_function(move |_, id: u32| {
            with_state(&s, |state| {
                Ok(state
                    .input
                    .gamepads
                    .connected
                    .get(&id)
                    .map(|pad| pad.name.clone()))
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "isDown",
        lua.create_function(move |_, (id, button): (u32, String)| {
            with_state(&s, |state| {
                anyhow::ensure!(
                    kairo_core::gamepad::BUTTONS.contains(&button.as_str()),
                    "unknown gamepad button '{button}'"
                );
                Ok(state
                    .input
                    .gamepads
                    .connected
                    .get(&id)
                    .is_some_and(|pad| pad.buttons.contains(&button)))
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "axis",
        lua.create_function(move |_, (id, axis): (u32, String)| {
            with_state(&s, |state| {
                anyhow::ensure!(
                    ["left_stick", "right_stick", "left_trigger", "right_trigger"]
                        .contains(&axis.as_str()),
                    "unknown gamepad axis"
                );
                let Some(pad) = state.input.gamepads.connected.get(&id) else {
                    return Ok((0.0, 0.0));
                };
                let value = match axis.as_str() {
                    "left_stick" => pad.left_stick,
                    "right_stick" => pad.right_stick,
                    "left_trigger" => [pad.left_trigger, 0.0],
                    _ => [pad.right_trigger, 0.0],
                };
                Ok((value[0], value[1]))
            })
        })?,
    )?;
    lua.globals().set("gamepad", module)
}
