use crate::state::{lua_error, with_state, SharedState};
use anyhow::ensure;
use glam::Vec2;
use kairo_core::{DrawCommand, FontHandle, Quad, Transform};
use mlua::{FromLua, Lua, UserData, Value, Variadic};

#[derive(Clone, Copy)]
struct FontResource(FontHandle);
impl UserData for FontResource {}

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let graphics: mlua::Table = lua.globals().get("graphics")?;
    let s = state.clone();
    graphics.set(
        "loadFont",
        lua.create_function(move |_, (path, size): (String, f32)| {
            with_state(&s, |state| Ok(FontResource(state.fonts.load(&path, size)?)))
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "releaseFont",
        lua.create_function(move |_, font: mlua::AnyUserData| {
            let font = font.borrow::<FontResource>()?.0;
            with_state(&s, |state| {
                ensure!(
                    !state.drawing,
                    "releaseFont must be called outside game.draw"
                );
                state.fonts.release(font, &mut state.assets)
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "measureText",
        lua.create_function(move |_, (font, text): (mlua::AnyUserData, String)| {
            let handle = font.borrow::<FontResource>()?.0;
            with_state(&s, |state| {
                let layout = state.fonts.layout(&mut state.assets, handle, &text)?;
                Ok((layout.width, layout.height))
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "print",
        lua.create_function(move |lua, args: Variadic<Value>| match args.as_slice() {
            [Value::UserData(font), text, x, y] => {
                let handle = font.borrow::<FontResource>()?.0;
                let text = String::from_lua(text.clone(), lua)?;
                let x = f32::from_lua(x.clone(), lua)?;
                let y = f32::from_lua(y.clone(), lua)?;
                with_state(&s, |state| {
                    state.require_draw()?;
                    kairo_core::finite("text position", &[x, y])?;
                    let layout = state.fonts.layout(&mut state.assets, handle, &text)?;
                    for glyph in layout.glyphs {
                        state.frame.push(DrawCommand::Quad(Quad {
                            texture: Some(glyph.texture),
                            size: Vec2::from(glyph.size),
                            uv_min: Vec2::from(glyph.uv_min),
                            uv_max: Vec2::from(glyph.uv_max),
                            transform: Transform {
                                position: Vec2::new(x, y) + Vec2::from(glyph.position),
                                ..Default::default()
                            },
                            camera: state.camera,
                            color: state.color,
                        }))?;
                    }
                    Ok(())
                })
            }
            [text, x, y] | [text, x, y, _] if !matches!(text, Value::UserData(_)) => {
                let text = String::from_lua(text.clone(), lua)?;
                let x = f32::from_lua(x.clone(), lua)?;
                let y = f32::from_lua(y.clone(), lua)?;
                let scale = args
                    .get(3)
                    .map(|v| f32::from_lua(v.clone(), lua))
                    .transpose()?
                    .unwrap_or(2.0);
                with_state(&s, |state| {
                    state.require_draw()?;
                    kairo_core::finite("text", &[x, y, scale])?;
                    ensure!(
                        scale > 0.0 && text.len() <= 4096,
                        "invalid bitmap text scale or text exceeds 4096 bytes"
                    );
                    state.frame.push(DrawCommand::Text {
                        text,
                        position: Vec2::new(x, y),
                        scale,
                        camera: state.camera,
                        color: state.color,
                    })
                })
            }
            _ => Err(lua_error(anyhow::anyhow!(
                "print expects (font, text, x, y) or (text, x, y[, bitmap_scale])"
            ))),
        })?,
    )?;
    Ok(())
}
