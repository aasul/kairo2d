use crate::state::{check_options, lua_error, with_state, SharedState};
use anyhow::ensure;
use glam::Vec2;
use kairo_assets::TextureFilter;
use kairo_core::{finite, Camera, Color, DrawCommand, PixelRect, Quad, TextureHandle, Transform};
use mlua::{AnyUserData, Lua, Table, UserData, UserDataMethods, Value};

#[derive(Clone, Copy)]
pub(crate) struct Texture {
    pub(crate) handle: TextureHandle,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl UserData for Texture {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getDimensions", |_, this, ()| Ok((this.width, this.height)));
    }
}

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let graphics = lua.create_table()?;
    let s = state.clone();
    graphics.set(
        "loadTexture",
        lua.create_function(move |_, path: String| {
            with_state(&s, |state| {
                let handle = state.assets.load_texture(&path)?;
                let texture = state.assets.texture(handle)?;
                Ok(Texture {
                    handle,
                    width: texture.width,
                    height: texture.height,
                })
            })
        })?,
    )?;

    let s = state.clone();
    graphics.set(
        "releaseTexture",
        lua.create_function(move |_, texture: AnyUserData| {
            let handle = texture.borrow::<Texture>()?.handle;
            with_state(&s, |state| {
                ensure!(
                    !state.drawing,
                    "releaseTexture must be called outside game.draw"
                );
                state.assets.release(handle)
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "setFilter",
        lua.create_function(move |_, (texture, mode): (AnyUserData, String)| {
            let handle = texture.borrow::<Texture>()?.handle;
            let filter = match mode.as_str() {
                "nearest" => TextureFilter::Nearest,
                "linear" => TextureFilter::Linear,
                _ => {
                    return Err(lua_error(anyhow::anyhow!(
                        "filter must be 'nearest' or 'linear'"
                    )))
                }
            };
            with_state(&s, |state| {
                ensure!(!state.drawing, "setFilter must be called outside game.draw");
                state.assets.set_filter(handle, filter)
            })
        })?,
    )?;

    let s = state.clone();
    graphics.set(
        "setViewport",
        lua.create_function(move |_, (x, y, width, height): (u32, u32, u32, u32)| {
            with_state(&s, |state| {
                state.require_draw()?;
                let rect = PixelRect {
                    x,
                    y,
                    width,
                    height,
                };
                ensure!(
                    width > 0
                        && height > 0
                        && rect.intersection(PixelRect::full(state.width, state.height))
                            == Some(rect),
                    "viewport must have positive size and fit inside the window"
                );
                state.frame.push(DrawCommand::Viewport(Some(rect)))?;
                state.viewport = Some(rect);
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "resetViewport",
        lua.create_function(move |_, ()| {
            with_state(&s, |state| {
                state.require_draw()?;
                state.frame.push(DrawCommand::Viewport(None))?;
                state.viewport = None;
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "setScissor",
        lua.create_function(move |_, (x, y, width, height): (u32, u32, u32, u32)| {
            with_state(&s, |state| {
                state.require_draw()?;
                let rect = PixelRect {
                    x,
                    y,
                    width,
                    height,
                };
                state.frame.push(DrawCommand::Scissor(Some(rect)))?;
                state.scissor = Some(rect);
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "resetScissor",
        lua.create_function(move |_, ()| {
            with_state(&s, |state| {
                state.require_draw()?;
                state.frame.push(DrawCommand::Scissor(None))?;
                state.scissor = None;
                Ok(())
            })
        })?,
    )?;

    let s = state.clone();
    graphics.set(
        "clear",
        lua.create_function(move |_, (r, g, b, a): (f32, f32, f32, Option<f32>)| {
            with_state(&s, |state| {
                state.require_draw()?;
                state.frame.clear = Color::new(r, g, b, a.unwrap_or(1.0))?;
                state.frame.commands.clear();
                // clear removes earlier draws but does not change the active coordinate system.
                if state.viewport.is_some() {
                    state.frame.push(DrawCommand::Viewport(state.viewport))?;
                }
                if state.scissor.is_some() {
                    state.frame.push(DrawCommand::Scissor(state.scissor))?;
                }
                Ok(())
            })
        })?,
    )?;

    let s = state.clone();
    graphics.set(
        "setColor",
        lua.create_function(move |_, (r, g, b, a): (f32, f32, f32, Option<f32>)| {
            with_state(&s, |state| {
                state.color = Color::new(r, g, b, a.unwrap_or(1.0))?;
                Ok(())
            })
        })?,
    )?;

    let s = state.clone();
    graphics.set(
        "rectangle",
        lua.create_function(
            move |_, (mode, x, y, w, h, line): (String, f32, f32, f32, f32, Option<f32>)| {
                with_state(&s, |state| {
                    state.require_draw()?;
                    let line = line.unwrap_or(1.0);
                    finite("rectangle", &[x, y, w, h, line])?;
                    ensure!(
                        w >= 0.0 && h >= 0.0,
                        "rectangle dimensions cannot be negative"
                    );
                    ensure!(line > 0.0, "line width must be positive");
                    ensure!(
                        mode == "fill" || mode == "line",
                        "rectangle mode must be 'fill' or 'line'"
                    );
                    if w == 0.0 || h == 0.0 {
                        return Ok(());
                    }
                    let mut add = |x, y, w, h| {
                        state.frame.push(DrawCommand::Quad(Quad {
                            texture: None,
                            size: Vec2::new(w, h),
                            uv_min: Vec2::ZERO,
                            uv_max: Vec2::ONE,
                            transform: Transform {
                                position: Vec2::new(x, y),
                                ..Default::default()
                            },
                            camera: state.camera,
                            color: state.color,
                        }))
                    };
                    if mode == "fill" {
                        add(x, y, w, h)?;
                    } else {
                        let line = line.min(w / 2.0).min(h / 2.0);
                        add(x, y, w, line)?;
                        add(x, y + h - line, w, line)?;
                        add(x, y + line, line, h - line * 2.0)?;
                        add(x + w - line, y + line, line, h - line * 2.0)?;
                    }
                    Ok(())
                })
            },
        )?,
    )?;

    let s = state.clone();
    graphics.set(
        "draw",
        lua.create_function(
            move |_, (texture, position, y): (AnyUserData, Value, Option<f32>)| {
                let (texture, animation_frame) =
                    if texture.is::<crate::animation::SpriteAnimation>() {
                        let sprite = texture.borrow::<crate::animation::SpriteAnimation>()?;
                        let frame = sprite.animation.frame();
                        (
                            sprite.texture,
                            Some([
                                frame.x as f32,
                                frame.y as f32,
                                frame.w as f32,
                                frame.h as f32,
                            ]),
                        )
                    } else {
                        (*texture.borrow::<Texture>()?, None)
                    };
                // Read Lua tables before borrowing engine state: a table can have metamethods.
                let (transform, mut source) =
                    draw_options(position, y, texture.width, texture.height)?;
                if let Some(region) = animation_frame {
                    source = region;
                }
                with_state(&s, |state| {
                    state.require_draw()?;
                    state.assets.texture(texture.handle)?;
                    let [x, y, w, h] = source;
                    finite(
                        "sprite transform",
                        &[
                            transform.position.x,
                            transform.position.y,
                            transform.scale.x,
                            transform.scale.y,
                            transform.origin.x,
                            transform.origin.y,
                            transform.rotation,
                            x,
                            y,
                            w,
                            h,
                        ],
                    )?;
                    ensure!(
                        x >= 0.0 && y >= 0.0 && w > 0.0 && h > 0.0,
                        "invalid sprite source rectangle"
                    );
                    ensure!(
                        x + w <= texture.width as f32 && y + h <= texture.height as f32,
                        "sprite source rectangle exceeds texture dimensions"
                    );
                    let dimensions = Vec2::new(texture.width as f32, texture.height as f32);
                    state.frame.push(DrawCommand::Quad(Quad {
                        texture: Some(texture.handle),
                        size: Vec2::new(w, h),
                        uv_min: Vec2::new(x, y) / dimensions,
                        uv_max: Vec2::new(x + w, y + h) / dimensions,
                        transform,
                        camera: state.camera,
                        color: state.color,
                    }))
                })
            },
        )?,
    )?;

    let s = state.clone();
    graphics.set(
        "setCamera",
        lua.create_function(
            move |_, (x, y, zoom, rotation): (f32, f32, Option<f32>, Option<f32>)| {
                with_state(&s, |state| {
                    let zoom = zoom.unwrap_or(1.0);
                    let rotation = rotation.unwrap_or(0.0);
                    finite("camera", &[x, y, zoom, rotation])?;
                    ensure!(zoom > 0.0, "camera zoom must be positive");
                    state.camera = Camera {
                        position: Vec2::new(x, y),
                        zoom,
                        rotation,
                    };
                    Ok(())
                })
            },
        )?,
    )?;
    let s = state.clone();
    graphics.set(
        "resetCamera",
        lua.create_function(move |_, ()| {
            with_state(&s, |state| {
                state.camera = Camera::default();
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "screenToWorld",
        lua.create_function(move |_, (x, y): (f32, f32)| {
            with_state(&s, |state| {
                finite("screen position", &[x, y])?;
                let origin = state
                    .viewport
                    .map(|rect| Vec2::new(rect.x as f32, rect.y as f32))
                    .unwrap_or(Vec2::ZERO);
                let p = state.camera.screen_to_world(Vec2::new(x, y) - origin);
                Ok((p.x, p.y))
            })
        })?,
    )?;
    let s = state.clone();
    graphics.set(
        "worldToScreen",
        lua.create_function(move |_, (x, y): (f32, f32)| {
            with_state(&s, |state| {
                finite("world position", &[x, y])?;
                let origin = state
                    .viewport
                    .map(|rect| Vec2::new(rect.x as f32, rect.y as f32))
                    .unwrap_or(Vec2::ZERO);
                let p = state.camera.world_to_screen(Vec2::new(x, y)) + origin;
                Ok((p.x, p.y))
            })
        })?,
    )?;
    lua.globals().set("graphics", graphics)
}

fn draw_options(
    position: Value,
    y: Option<f32>,
    width: u32,
    height: u32,
) -> mlua::Result<(Transform, [f32; 4])> {
    let mut source = [0.0, 0.0, width as f32, height as f32];
    let transform = match position {
        Value::Table(table) => {
            if y.is_some() {
                return Err(lua_error(anyhow::anyhow!("draw accepts a table OR x, y")));
            }
            check_options(
                &table,
                &[
                    "x", "y", "rotation", "scale_x", "scale_y", "origin_x", "origin_y", "source",
                ],
            )?;
            if let Some(region) = table.get::<Option<Table>>("source")? {
                check_options(&region, &["x", "y", "width", "height"])?;
                source = [
                    region.get("x")?,
                    region.get("y")?,
                    region.get("width")?,
                    region.get("height")?,
                ];
            }
            Transform {
                position: Vec2::new(
                    table.get::<Option<f32>>("x")?.unwrap_or(0.0),
                    table.get::<Option<f32>>("y")?.unwrap_or(0.0),
                ),
                scale: Vec2::new(
                    table.get::<Option<f32>>("scale_x")?.unwrap_or(1.0),
                    table.get::<Option<f32>>("scale_y")?.unwrap_or(1.0),
                ),
                origin: Vec2::new(
                    table.get::<Option<f32>>("origin_x")?.unwrap_or(0.0),
                    table.get::<Option<f32>>("origin_y")?.unwrap_or(0.0),
                ),
                rotation: table.get::<Option<f32>>("rotation")?.unwrap_or(0.0),
            }
        }
        Value::Integer(x) => Transform {
            position: Vec2::new(
                x as f32,
                y.ok_or_else(|| lua_error(anyhow::anyhow!("draw(texture, x, y) requires y")))?,
            ),
            ..Default::default()
        },
        Value::Number(x) => Transform {
            position: Vec2::new(
                x as f32,
                y.ok_or_else(|| lua_error(anyhow::anyhow!("draw(texture, x, y) requires y")))?,
            ),
            ..Default::default()
        },
        _ => {
            return Err(lua_error(anyhow::anyhow!(
                "draw expects a texture and either x, y or an options table"
            )))
        }
    };
    Ok((transform, source))
}
