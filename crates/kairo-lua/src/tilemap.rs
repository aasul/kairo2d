use crate::state::{with_state, SharedState};
use anyhow::{ensure, Context};
use glam::Vec2;
use kairo_assets::tilemap::TileMap;
use kairo_core::{DrawCommand, Quad, Transform};
use mlua::{Lua, LuaSerdeExt, UserData, UserDataMethods};
use std::collections::HashMap;
use std::rc::Rc;

struct MapResource {
    map: Rc<TileMap>,
    state: SharedState,
    visibility: HashMap<String, bool>,
    opacity: HashMap<String, f32>,
    elapsed: f64,
    path: String,
}

impl UserData for MapResource {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getSize", |_, this, ()| {
            Ok((
                this.map.width,
                this.map.height,
                this.map.tile_width,
                this.map.tile_height,
            ))
        });
        methods.add_method("getLayers", |lua, this, ()| {
            lua.create_sequence_from(this.map.layers.iter().map(|layer| layer.name.clone()))
        });
        methods.add_method("getTile", |_, this, (name, x, y): (String, u32, u32)| {
            let layer = this
                .map
                .layers
                .iter()
                .find(|layer| layer.name == name)
                .ok_or_else(|| mlua::Error::RuntimeError(format!("unknown tile layer '{name}'")))?;
            Ok(layer.tile(x, y))
        });
        methods.add_method_mut(
            "setLayerVisible",
            |_, this, (name, visible): (String, bool)| {
                if !this.map.layers.iter().any(|layer| layer.name == name) {
                    return Err(mlua::Error::RuntimeError(format!(
                        "unknown tile layer '{name}'"
                    )));
                }
                this.visibility.insert(name, visible);
                Ok(())
            },
        );
        methods.add_method("getObjects", |lua, this, ()| {
            let objects = lua.create_table()?;
            for (index, object) in this.map.objects.iter().enumerate() {
                let value = lua.create_table()?;
                value.set("id", object.id)?;
                value.set("name", object.name.clone())?;
                value.set("type", object.kind.clone())?;
                value.set("x", object.x)?;
                value.set("y", object.y)?;
                value.set("layer", object.layer.clone())?;
                value.set("properties", lua.to_value(&object.properties)?)?;
                value.set("width", object.width)?;
                value.set("height", object.height)?;
                value.set("rotation", object.rotation)?;
                objects.set(index + 1, value)?;
            }
            Ok(objects)
        });
        methods.add_method("getProperties", |lua, this, ()| {
            lua.to_value(&this.map.properties)
        });
        methods.add_method("getLayerProperties", |lua, this, name: String| {
            let layer = this
                .map
                .layers
                .iter()
                .find(|v| v.name == name)
                .ok_or_else(|| mlua::Error::RuntimeError("unknown tile layer".into()))?;
            lua.to_value(&layer.properties)
        });
        methods.add_method("getTileProperties", |lua, this, gid: u32| {
            lua.to_value(
                &this
                    .map
                    .tile_properties(gid)
                    .map_err(crate::state::lua_error)?,
            )
        });
        methods.add_method("getCollisionRects", |lua, this, layer: Option<String>| {
            lua.to_value(
                &this
                    .map
                    .collision_rects(layer.as_deref())
                    .map_err(crate::state::lua_error)?,
            )
        });
        methods.add_method("findObject", |lua, this, name: String| {
            lua.to_value(&this.map.objects.iter().find(|v| v.name == name))
        });
        methods.add_method_mut("update", |_, this, dt: f64| {
            if !dt.is_finite() || !(0.0..=1.0).contains(&dt) {
                return Err(mlua::Error::RuntimeError(
                    "tilemap delta must be 0..1 second".into(),
                ));
            }
            this.elapsed += dt;
            Ok(())
        });
        methods.add_method_mut(
            "setLayerOpacity",
            |_, this, (name, value): (String, f32)| {
                if !value.is_finite()
                    || !(0.0..=1.0).contains(&value)
                    || !this.map.layers.iter().any(|v| v.name == name)
                {
                    return Err(mlua::Error::RuntimeError("invalid layer or opacity".into()));
                }
                this.opacity.insert(name, value);
                Ok(())
            },
        );
        methods.add_method_mut("reload", |_, this, ()| {
            let map = with_state(&this.state, |state| {
                let map = Rc::new(TileMap::load(&state.fs, &mut state.assets, &this.path)?);
                state
                    .tilemaps
                    .insert(state.fs.resolve(&this.path)?, map.clone());
                Ok(map)
            })?;
            this.map = map;
            Ok(())
        });
        methods.add_method("draw", |_, this, (x, y): (Option<f32>, Option<f32>)| {
            with_state(&this.state, |state| {
                state.require_draw()?;
                let offset = Vec2::new(x.unwrap_or(0.0), y.unwrap_or(0.0));
                kairo_core::finite("tilemap position", &[offset.x, offset.y])?;
                let view = state
                    .viewport
                    .unwrap_or(kairo_core::PixelRect::full(state.width, state.height));
                let corners = [
                    Vec2::ZERO,
                    Vec2::new(view.width as f32, 0.0),
                    Vec2::new(0.0, view.height as f32),
                    Vec2::new(view.width as f32, view.height as f32),
                ]
                .map(|point| state.camera.screen_to_world(point));
                let min = corners
                    .iter()
                    .copied()
                    .fold(Vec2::splat(f32::INFINITY), Vec2::min);
                let max = corners
                    .iter()
                    .copied()
                    .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
                let tile = Vec2::new(this.map.tile_width as f32, this.map.tile_height as f32);
                for layer in &this.map.layers {
                    let opacity = this
                        .opacity
                        .get(&layer.name)
                        .copied()
                        .unwrap_or(layer.opacity);
                    if !this
                        .visibility
                        .get(&layer.name)
                        .copied()
                        .unwrap_or(layer.visible)
                        || opacity == 0.0
                    {
                        continue;
                    }
                    let origin = offset
                        + Vec2::from(layer.offset)
                        + (state.camera.position - Vec2::from(this.map.parallax_origin))
                            * (Vec2::ONE - Vec2::from(layer.parallax));
                    let start = ((min - origin) / tile).floor().max(Vec2::ZERO);
                    let end = ((max - origin) / tile)
                        .ceil()
                        .min(Vec2::new(layer.width as f32, layer.height as f32));
                    for ty in start.y as u32..end.y.max(0.0) as u32 {
                        for tx in start.x as u32..end.x.max(0.0) as u32 {
                            let raw = layer
                                .tile(tx, ty)
                                .context("tile culling produced an invalid cell")?;
                            if raw == 0 {
                                continue;
                            }
                            let (set, source, flip_x, flip_y) =
                                this.map.tile_source_at(raw, this.elapsed)?;
                            state.assets.texture(set.texture)?;
                            let dimensions =
                                Vec2::new(set.texture_size[0] as f32, set.texture_size[1] as f32);
                            let mut uv_min =
                                Vec2::new(source[0] as f32, source[1] as f32) / dimensions;
                            let mut uv_max = Vec2::new(
                                (source[0] + source[2]) as f32,
                                (source[1] + source[3]) as f32,
                            ) / dimensions;
                            if flip_x {
                                std::mem::swap(&mut uv_min.x, &mut uv_max.x);
                            }
                            if flip_y {
                                std::mem::swap(&mut uv_min.y, &mut uv_max.y);
                            }
                            let mut color = state.color;
                            color.0[3] *= opacity;
                            state.frame.push(DrawCommand::Quad(Quad {
                                texture: Some(set.texture),
                                size: tile,
                                uv_min,
                                uv_max,
                                transform: Transform {
                                    position: origin + Vec2::new(tx as f32, ty as f32) * tile,
                                    ..Default::default()
                                },
                                camera: state.camera,
                                color,
                            }))?;
                        }
                    }
                }
                Ok(())
            })
        });
    }
}

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let s = state.clone();
    module.set(
        "load",
        lua.create_function(move |_, path: String| {
            let map = with_state(&s, |state| {
                let canonical = state.fs.resolve(&path)?;
                if let Some(map) = state.tilemaps.get(&canonical) {
                    return Ok(map.clone());
                }
                ensure!(state.tilemaps.len() < 32, "tilemap cache limit is 32 maps");
                let map = Rc::new(TileMap::load(&state.fs, &mut state.assets, &path)?);
                state.tilemaps.insert(canonical, map.clone());
                Ok(map)
            })?;
            Ok(MapResource {
                map,
                state: s.clone(),
                visibility: HashMap::new(),
                opacity: HashMap::new(),
                elapsed: 0.0,
                path,
            })
        })?,
    )?;
    lua.globals().set("tilemap", module)
}
