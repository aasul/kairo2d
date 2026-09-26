use crate::graphics::Texture;
use crate::state::{lua_error, with_state, SharedState};
use kairo_core::animation::{Animation, AnimationFile, AnimationFrame};
use mlua::{AnyUserData, Lua, Table, UserData, UserDataMethods};

pub(crate) struct SpriteAnimation {
    pub texture: Texture,
    pub animation: Animation,
}

impl UserData for SpriteAnimation {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("update", |_, this, dt: f64| {
            this.animation.update(dt).map_err(lua_error)
        });
        methods.add_method_mut("setLooping", |_, this, value: bool| {
            this.animation.set_looping(value);
            Ok(())
        });
        methods.add_method_mut("setSpeed", |_, this, value: f64| {
            this.animation.set_speed(value).map_err(lua_error)
        });
        methods.add_method_mut("pause", |_, this, ()| {
            this.animation.set_paused(true);
            Ok(())
        });
        methods.add_method_mut("resume", |_, this, ()| {
            this.animation.set_paused(false);
            Ok(())
        });
        methods.add_method_mut("restart", |_, this, ()| {
            this.animation.restart();
            Ok(())
        });
        methods.add_method("isFinished", |_, this, ()| Ok(this.animation.finished()));
        methods.add_method("getFrame", |_, this, ()| {
            Ok(this.animation.frame_index() + 1)
        });
    }
}

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set(
        "new",
        lua.create_function(move |_, (texture, frames): (AnyUserData, Table)| {
            let texture = *texture.borrow::<Texture>()?;
            let mut parsed = Vec::new();
            for frame in frames.sequence_values::<Table>() {
                let frame = frame?;
                crate::state::check_options(&frame, &["x", "y", "w", "h", "duration"])?;
                if parsed.len() >= 4096 {
                    return Err(lua_error(anyhow::anyhow!("too many animation frames")));
                }
                parsed.push(AnimationFrame {
                    x: frame.get("x")?,
                    y: frame.get("y")?,
                    w: frame.get("w")?,
                    h: frame.get("h")?,
                    duration: frame.get("duration")?,
                });
            }
            with_state(&s, |state| {
                state.assets.texture(texture.handle)?;
                Ok(SpriteAnimation {
                    texture,
                    animation: Animation::new(parsed, texture.width, texture.height)?,
                })
            })
        })?,
    )?;
    let s = state.clone();
    module.set(
        "load",
        lua.create_function(move |_, path: String| {
            with_state(&s, |state| {
                let file: AnimationFile = serde_json::from_slice(&state.fs.read(&path)?)?;
                let texture_path = kairo_assets::tilemap::relative_asset(&path, &file.texture)?;
                let handle = state.assets.load_texture(&texture_path)?;
                let asset = state.assets.texture(handle)?;
                let texture = Texture {
                    handle,
                    width: asset.width,
                    height: asset.height,
                };
                let mut animation = Animation::new(file.frames, texture.width, texture.height)?;
                animation.set_looping(file.looping);
                Ok(SpriteAnimation { texture, animation })
            })
        })?,
    )?;
    lua.globals().set("animation", module)
}
