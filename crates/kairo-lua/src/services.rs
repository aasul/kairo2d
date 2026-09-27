use crate::state::{with_state, SharedState, WindowCommand};
use anyhow::ensure;
use mlua::{Lua, LuaSerdeExt};

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    lua.globals().set(
        "print",
        lua.create_function(|lua, args: mlua::Variadic<mlua::Value>| {
            let tostring: mlua::Function = lua.globals().get("tostring")?;
            let mut text = String::new();
            for value in args {
                let value: mlua::String = tostring.call(value)?;
                if !text.is_empty() {
                    text.push('\t');
                }
                let value = value.to_string_lossy();
                let remaining = 8192_usize.saturating_sub(text.len());
                let mut end = value.len().min(remaining);
                while !value.is_char_boundary(end) {
                    end -= 1;
                }
                text.push_str(&value[..end]);
                if text.len() >= 8192 {
                    break;
                }
            }
            log::info!("{text}");
            Ok(())
        })?,
    )?;
    let keyboard = lua.create_table()?;
    let s = state.clone();
    keyboard.set(
        "isDown",
        lua.create_function(move |_, key: String| with_state(&s, |s| Ok(s.input.key_down(&key))))?,
    )?;
    lua.globals().set("keyboard", keyboard)?;

    let mouse = lua.create_table()?;
    let s = state.clone();
    mouse.set(
        "position",
        lua.create_function(move |_, ()| {
            with_state(&s, |s| Ok((s.input.mouse_x, s.input.mouse_y)))
        })?,
    )?;
    let s = state.clone();
    mouse.set(
        "isDown",
        lua.create_function(move |_, button: u8| {
            with_state(&s, |s| {
                ensure!((1..=5).contains(&button), "mouse button must be in 1..=5");
                Ok(s.input.mouse_down(button))
            })
        })?,
    )?;
    lua.globals().set("mouse", mouse)?;

    let window = lua.create_table()?;
    let s = state.clone();
    window.set(
        "getSize",
        lua.create_function(move |_, ()| with_state(&s, |s| Ok((s.width, s.height))))?,
    )?;
    let s = state.clone();
    window.set(
        "getPhysicalSize",
        lua.create_function(move |_, ()| {
            with_state(&s, |state| {
                Ok((state.physical_size[0], state.physical_size[1]))
            })
        })?,
    )?;
    let s = state.clone();
    window.set(
        "setTitle",
        lua.create_function(move |_, title: String| {
            with_state(&s, |s| {
                ensure!(
                    !title.trim().is_empty() && title.len() <= 256,
                    "title must contain 1..=256 UTF-8 bytes"
                );
                s.window_command(WindowCommand::Title(title))
            })
        })?,
    )?;
    let s = state.clone();
    window.set(
        "setSize",
        lua.create_function(move |_, (width, height): (u32, u32)| {
            with_state(&s, |s| {
                ensure!(
                    (1..=8192).contains(&width) && (1..=8192).contains(&height),
                    "window dimensions must be in 1..=8192"
                );
                s.window_command(WindowCommand::Size(width, height))
            })
        })?,
    )?;
    let s = state.clone();
    window.set(
        "setVsync",
        lua.create_function(move |_, enabled: bool| {
            with_state(&s, |s| s.window_command(WindowCommand::Vsync(enabled)))
        })?,
    )?;
    let s = state.clone();
    window.set(
        "close",
        lua.create_function(move |_, ()| {
            with_state(&s, |s| s.window_command(WindowCommand::Close))
        })?,
    )?;
    lua.globals().set("window", window)?;

    let timer = lua.create_table()?;
    let s = state.clone();
    timer.set(
        "getDelta",
        lua.create_function(move |_, ()| with_state(&s, |s| Ok(s.delta)))?,
    )?;
    let s = state.clone();
    timer.set(
        "getTime",
        lua.create_function(move |_, ()| with_state(&s, |s| Ok(s.elapsed)))?,
    )?;
    let s = state.clone();
    timer.set(
        "getFPS",
        lua.create_function(move |_, ()| with_state(&s, |s| Ok(s.fps)))?,
    )?;
    lua.globals().set("timer", timer)?;

    let filesystem = lua.create_table()?;
    let s = state.clone();
    filesystem.set(
        "read",
        lua.create_function(move |lua, path: String| {
            let bytes = with_state(&s, |s| s.fs.read(&path))?;
            lua.create_string(bytes)
        })?,
    )?;
    let s = state.clone();
    filesystem.set(
        "exists",
        lua.create_function(move |_, path: String| with_state(&s, |s| s.fs.exists(&path)))?,
    )?;
    let s = state.clone();
    filesystem.set(
        "list",
        lua.create_function(move |lua, path: String| {
            let names = with_state(&s, |s| s.fs.list(&path))?;
            lua.create_sequence_from(names)
        })?,
    )?;

    for (name, is_toml) in [("readJson", false), ("readToml", true)] {
        let s = state.clone();
        filesystem.set(
            name,
            lua.create_function(move |lua, path: String| {
                let value: serde_json::Value = with_state(&s, |state| {
                    let source = state.fs.read_text(&path)?;
                    ensure!(
                        source.len() <= 1024 * 1024,
                        "structured data file exceeds 1 MiB"
                    );
                    if is_toml {
                        Ok(serde_json::to_value(toml::from_str::<toml::Value>(
                            &source,
                        )?)?)
                    } else {
                        Ok(serde_json::from_str(&source)?)
                    }
                })?;
                lua.to_value(&value)
            })?,
        )?;
    }
    lua.globals().set("filesystem", filesystem)?;

    let assets = lua.create_table()?;
    let s = state.clone();
    assets.set(
        "stats",
        lua.create_function(move |lua, ()| {
            let values = with_state(&s, |s| {
                Ok([
                    s.assets.texture_count(),
                    s.assets.texture_bytes(),
                    s.audio.sound_count(),
                    s.audio.decoded_bytes(),
                    s.audio.voice_count(),
                    s.physics.body_count(),
                ])
            })?;
            let result = lua.create_table()?;
            for (name, value) in [
                "textures",
                "texture_bytes",
                "sounds",
                "sound_bytes",
                "voices",
                "bodies",
            ]
            .into_iter()
            .zip(values)
            {
                result.set(name, value)?;
            }
            Ok(result)
        })?,
    )?;
    lua.globals().set("assets", assets)
}
