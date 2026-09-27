use crate::state::{with_state, SharedState};
use anyhow::ensure;
use kairo_core::debugui::{DebugWindow, Response, Widget};
use mlua::{Function, Lua};

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set(
        "window",
        lua.create_function(move |_, (title, body): (String, Function)| {
            with_state(&s, |state| {
                let ui = &mut state.debug_ui;
                ensure!(
                    ui.building && ui.current.is_none(),
                    "debugui windows must be top-level in game.debugUI"
                );
                ensure!(
                    !title.is_empty() && title.len() <= 128 && ui.windows.len() < 16,
                    "invalid debug window title/count"
                );
                ensure!(
                    !ui.windows.iter().any(|w| w.title == title),
                    "debug window titles must be unique"
                );
                ui.current = Some(ui.windows.len());
                ui.windows.push(DebugWindow {
                    title,
                    widgets: Vec::new(),
                });
                Ok(())
            })?;
            let result = body.call::<()>(());
            with_state(&s, |state| {
                state.debug_ui.current = None;
                Ok(())
            })?;
            result
        })?,
    )?;
    let s = state.clone();
    module.set(
        "text",
        lua.create_function(move |_, text: String| {
            with_state(&s, |state| {
                ensure!(text.len() <= 4096, "debug text exceeds 4096 bytes");
                state.debug_ui.key("")?;
                state.debug_ui.push(Widget::Text(text))
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "button",
        lua.create_function(move |_, label: String| {
            with_state(&s, |state| {
                let ui = &mut state.debug_ui;
                let key = ui.key(&label)?;
                let clicked = matches!(ui.responses.remove(&key), Some(Response::Click));
                ui.push(Widget::Button { label, key })?;
                Ok(clicked)
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "checkbox",
        lua.create_function(move |_, (label, mut value): (String, bool)| {
            with_state(&s, |state| {
                let ui = &mut state.debug_ui;
                let key = ui.key(&label)?;
                if let Some(Response::Bool(next)) = ui.responses.remove(&key) {
                    value = next;
                }
                ui.push(Widget::Checkbox { label, key, value })?;
                Ok(value)
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "slider",
        lua.create_function(
            move |_, (label, mut value, min, max): (String, f64, f64, f64)| {
                with_state(&s, |state| {
                    ensure!(
                        value.is_finite() && min.is_finite() && max.is_finite() && min <= max,
                        "invalid debug slider range/value"
                    );
                    let ui = &mut state.debug_ui;
                    let key = ui.key(&label)?;
                    if let Some(Response::Number(next)) = ui.responses.remove(&key) {
                        value = next;
                    }
                    value = value.clamp(min, max);
                    ui.push(Widget::Slider {
                        label,
                        key,
                        value,
                        min,
                        max,
                    })?;
                    Ok(value)
                })
            },
        )?,
    )?;
    let s = state.clone();
    module.set(
        "wantsMouse",
        lua.create_function(move |_, ()| with_state(&s, |state| Ok(state.debug_ui.wants_mouse)))?,
    )?;
    lua.globals().set("debugui", module)
}
