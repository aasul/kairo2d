use crate::state::{with_state, SharedState};
use kairo_core::{physics::BodyKind, BodyHandle};
use mlua::{AnyUserData, Lua, UserData, UserDataMethods};

struct Body {
    handle: BodyHandle,
    state: SharedState,
}

impl UserData for Body {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getPosition", |_, body, ()| {
            with_state(&body.state, |s| s.physics.position(body.handle))
        });
        methods.add_method("setPosition", |_, body, (x, y): (f32, f32)| {
            with_state(&body.state, |s| s.physics.set_position(body.handle, x, y))
        });
        methods.add_method("getRotation", |_, body, ()| {
            with_state(&body.state, |s| s.physics.rotation(body.handle))
        });
        methods.add_method("setRotation", |_, body, angle: f32| {
            with_state(&body.state, |s| s.physics.set_rotation(body.handle, angle))
        });
        methods.add_method("getVelocity", |_, body, ()| {
            with_state(&body.state, |s| s.physics.velocity(body.handle))
        });
        methods.add_method("setVelocity", |_, body, (x, y): (f32, f32)| {
            with_state(&body.state, |s| s.physics.set_velocity(body.handle, x, y))
        });
        methods.add_method("applyImpulse", |_, body, (x, y): (f32, f32)| {
            with_state(&body.state, |s| s.physics.apply_impulse(body.handle, x, y))
        });
        methods.add_method("isTouching", |_, body, other: AnyUserData| {
            let other = other.borrow::<Body>()?;
            with_state(&body.state, |s| {
                s.physics.is_touching(body.handle, other.handle)
            })
        });
        methods.add_method("destroy", |_, body, ()| {
            with_state(&body.state, |s| s.physics.destroy(body.handle))
        });
    }
}

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let physics = lua.create_table()?;
    let s = state.clone();
    physics.set(
        "newRectangle",
        lua.create_function(
            move |_, (kind, x, y, width, height): (String, f32, f32, f32, f32)| {
                let handle = with_state(&s, |s| {
                    s.physics
                        .rectangle(BodyKind::parse(&kind)?, x, y, width, height)
                })?;
                Ok(Body {
                    handle,
                    state: s.clone(),
                })
            },
        )?,
    )?;
    let s = state.clone();
    physics.set(
        "newCircle",
        lua.create_function(move |_, (kind, x, y, radius): (String, f32, f32, f32)| {
            let handle = with_state(&s, |s| {
                s.physics.circle(BodyKind::parse(&kind)?, x, y, radius)
            })?;
            Ok(Body {
                handle,
                state: s.clone(),
            })
        })?,
    )?;
    let s = state.clone();
    physics.set(
        "setGravity",
        lua.create_function(move |_, (x, y): (f32, f32)| {
            with_state(&s, |s| s.physics.set_gravity(x, y))
        })?,
    )?;
    lua.globals().set("physics", physics)
}
