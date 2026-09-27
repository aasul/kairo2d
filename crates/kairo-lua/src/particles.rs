use crate::{
    graphics::Texture,
    state::{lua_error, with_state, SharedState},
};
use kairo_core::{
    particles::{Emitter, EmitterConfig, ParticlePreset},
    Color, DrawCommand, Quad, Transform,
};
use mlua::{Lua, LuaSerdeExt, Table, UserData, UserDataMethods, Value};
use std::{cell::Cell, rc::Rc};

struct ParticleEmitter {
    emitter: Emitter,
    texture: Option<Texture>,
    state: SharedState,
    budget: Rc<Cell<usize>>,
    live_count: Rc<Cell<usize>>,
}
impl ParticleEmitter {
    fn sync_count(&self, before: usize) {
        let total = self.live_count.get().saturating_sub(before);
        self.live_count
            .set(total.saturating_add(self.emitter.particles().len()));
    }
}
impl Drop for ParticleEmitter {
    fn drop(&mut self) {
        self.budget.set(self.budget.get().saturating_sub(1));
        self.live_count.set(
            self.live_count
                .get()
                .saturating_sub(self.emitter.particles().len()),
        );
    }
}
impl UserData for ParticleEmitter {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("update", |_, this, dt: f32| {
            let before = this.emitter.particles().len();
            this.emitter.update(dt).map_err(lua_error)?;
            this.sync_count(before);
            Ok(())
        });
        methods.add_method_mut("emit", |_, this, count: usize| {
            let before = this.emitter.particles().len();
            let emitted = this.emitter.emit(count);
            this.sync_count(before);
            Ok(emitted)
        });
        methods.add_method_mut("setPosition", |_, this, (x, y): (f32, f32)| {
            this.emitter.set_position(x, y).map_err(lua_error)
        });
        methods.add_method_mut("clear", |_, this, ()| {
            let before = this.emitter.particles().len();
            this.emitter.clear();
            this.sync_count(before);
            Ok(())
        });
        methods.add_method("count", |_, this, ()| Ok(this.emitter.particles().len()));
        methods.add_method("draw", |_, this, ()| {
            with_state(&this.state, |state| {
                state.require_draw()?;
                if let Some(texture) = this.texture {
                    state.assets.texture(texture.handle)?;
                }
                for particle in this.emitter.particles() {
                    let (size, mut color) = this.emitter.appearance(particle);
                    for (a, b) in color.iter_mut().zip(state.color.0) {
                        *a *= b;
                    }
                    state.frame.push(DrawCommand::Quad(Quad {
                        texture: this.texture.map(|t| t.handle),
                        size: glam::Vec2::splat(size),
                        uv_min: glam::Vec2::ZERO,
                        uv_max: glam::Vec2::ONE,
                        transform: Transform {
                            position: particle.position,
                            origin: glam::Vec2::splat(size * 0.5),
                            rotation: particle.rotation,
                            ..Default::default()
                        },
                        camera: state.camera,
                        color: Color(color),
                    }))?;
                }
                Ok(())
            })
        });
    }
}
fn create(
    state: &SharedState,
    config: EmitterConfig,
    texture: Option<Texture>,
) -> mlua::Result<ParticleEmitter> {
    let emitter = Emitter::new(config).map_err(lua_error)?;
    let (budget, live_count) = with_state(state, |state| {
        anyhow::ensure!(
            state.particle_count.get() < 64,
            "particle emitter limit reached (64)"
        );
        state.particle_count.set(state.particle_count.get() + 1);
        Ok((
            state.particle_count.clone(),
            state.particle_live_count.clone(),
        ))
    })?;
    Ok(ParticleEmitter {
        emitter,
        texture,
        state: state.clone(),
        budget,
        live_count,
    })
}
pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set(
        "new",
        lua.create_function(move |lua, options: Table| {
            let table = lua.create_table()?;
            let mut texture = None;
            for pair in options.pairs::<String, Value>() {
                let (key, value) = pair?;
                if key == "texture" {
                    if let Value::UserData(value) = value {
                        texture = Some(*value.borrow::<Texture>()?);
                    } else if !matches!(value, Value::Nil) {
                        return Err(lua_error(anyhow::anyhow!(
                            "texture must be a texture handle"
                        )));
                    }
                } else {
                    table.set(key, value)?;
                }
            }
            let config: EmitterConfig = lua.from_value(Value::Table(table))?;
            create(&s, config, texture)
        })?,
    )?;
    let s = state.clone();
    module.set(
        "load",
        lua.create_function(move |_, path: String| {
            let (preset, texture) = with_state(&s, |state| {
                let preset: ParticlePreset = toml::from_str(&state.fs.read_text(&path)?)?;
                let texture = if let Some(relative) = &preset.texture {
                    let relative = kairo_assets::tilemap::relative_asset(&path, relative)?;
                    let handle = state.assets.load_texture(&relative)?;
                    let asset = state.assets.texture(handle)?;
                    Some(Texture {
                        handle,
                        width: asset.width,
                        height: asset.height,
                    })
                } else {
                    None
                };
                Ok((preset, texture))
            })?;
            create(&s, preset.emitter, texture)
        })?,
    )?;
    lua.globals().set("particles", module)
}
