use crate::state::{lua_error, with_state, SharedState};
use anyhow::{ensure, Result};
use kairo_core::{config::ReplayConfig, physics::PhysicsSnapshot, InputState};
use kairo_replay::{Rng, Timeline};
use mlua::{Lua, LuaSerdeExt, Table, Value};
use serde::{Deserialize, Serialize};

pub enum ReplayRequest {
    Bookmark(u64),
    Rewind(f64),
    Seek(usize),
    SeekTime(f64),
    Pause,
    Resume,
    Step,
}

pub struct ReplayState {
    pub timeline: Timeline,
    pub bookmarks: kairo_core::bookmarks::Bookmarks,
    pub enabled: bool,
    pub paused: bool,
    pub step: bool,
    pub pending: Option<ReplayRequest>,
    interval: f64,
    accumulator: f64,
}

impl ReplayState {
    pub fn new(config: &ReplayConfig) -> Result<Self> {
        Ok(Self {
            timeline: Timeline::new(config.seconds, config.memory_mib * 1024 * 1024)?,
            bookmarks: Default::default(),
            enabled: config.enabled,
            paused: false,
            step: false,
            pending: None,
            interval: 1.0 / config.frequency as f64,
            accumulator: 0.0,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    registered: serde_json::Value,
    scenes: Vec<String>,
    input: InputState,
    actions: kairo_core::actions::Actions,
    physics: PhysicsSnapshot,
    rng: Rng,
    frame: u64,
    time: f64,
    delta: f32,
}

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let backend = lua.create_table()?;
    let s = state.clone();
    backend.set(
        "store",
        lua.create_function(move |lua, value: Value| {
            let registered = lua.from_value(value)?;
            let scenes = scene_stack(lua)?;
            with_state(&s, |state| {
                let snapshot = Snapshot {
                    registered,
                    scenes,
                    input: state.input.clone(),
                    actions: state.actions.clone(),
                    physics: state.physics.snapshot()?,
                    rng: state.rng.clone(),
                    frame: state.frame_number,
                    time: state.elapsed,
                    delta: state.delta,
                };
                let bytes = serde_json::to_vec(&snapshot)?;
                state
                    .replay
                    .timeline
                    .capture(state.frame_number, state.elapsed, bytes)
            })
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "clear",
        lua.create_function(move |_, ()| {
            with_state(&s, |state| {
                state.replay.timeline.clear();
                state.replay.bookmarks.clear();
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "enable",
        lua.create_function(move |_, enabled: bool| {
            with_state(&s, |state| {
                state.replay.enabled = enabled;
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "request",
        lua.create_function(move |_, (kind, amount): (String, f64)| {
            with_state(&s, |state| {
                ensure!(
                    amount.is_finite() && amount >= 0.0,
                    "replay amount must be finite and nonnegative"
                );
                let request = match kind.as_str() {
                    "rewind" => ReplayRequest::Rewind(amount),
                    "seek" => {
                        ensure!(
                            amount >= 1.0 && amount.fract() == 0.0,
                            "replay index starts at 1"
                        );
                        ReplayRequest::Seek(amount as usize - 1)
                    }
                    "pause" => ReplayRequest::Pause,
                    "resume" => ReplayRequest::Resume,
                    "step" => ReplayRequest::Step,
                    _ => anyhow::bail!("unknown replay operation"),
                };
                state.replay.pending = Some(request);
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "restore",
        lua.create_function(move |lua, ()| {
            let scenes = scene_stack(lua)?;
            let snapshot = with_state(&s, |state| {
                let Some(request) = state.replay.pending.take() else {
                    return Ok(None);
                };
                let from_bookmark = matches!(&request, ReplayRequest::Bookmark(_));
                let bytes = match request {
                    ReplayRequest::Bookmark(id) => state.replay.bookmarks.snapshot(id)?,
                    ReplayRequest::Pause => {
                        state.replay.paused = true;
                        return Ok(None);
                    }
                    ReplayRequest::Resume => {
                        state.replay.timeline.branch(state.frame_number);
                        state.replay.paused = false;
                        state.input.release_all();
                        return Ok(None);
                    }
                    ReplayRequest::Step => {
                        state.replay.timeline.branch(state.frame_number);
                        state.replay.paused = true;
                        state.replay.step = true;
                        return Ok(None);
                    }
                    ReplayRequest::Rewind(seconds) => {
                        state
                            .replay
                            .timeline
                            .at_or_before((state.elapsed - seconds).max(0.0))?
                            .1
                    }
                    ReplayRequest::SeekTime(time) => state.replay.timeline.at_or_before(time)?.1,
                    ReplayRequest::Seek(index) => state.replay.timeline.at_index(index)?.1,
                };
                let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
                ensure!(
                    snapshot.scenes == scenes,
                    "replay scene stack differs; switch to a compatible scene before restoring"
                );
                let replacement = if from_bookmark {
                    Some(
                        state
                            .replay
                            .timeline
                            .replacement(snapshot.frame, snapshot.time, bytes)?,
                    )
                } else {
                    None
                };
                state.physics.restore(&snapshot.physics)?;
                if let Some(replacement) = replacement {
                    state.replay.timeline = replacement;
                }
                state.audio.stop_all();
                state.input = snapshot.input;
                state.actions = snapshot.actions;
                state.rng = snapshot.rng;
                state.frame_number = snapshot.frame;
                state.elapsed = snapshot.time;
                state.delta = snapshot.delta;
                state.replay.paused = true;
                Ok(Some(snapshot.registered))
            })?;
            match snapshot {
                Some(snapshot) => lua.to_value(&snapshot),
                None => Ok(Value::Nil),
            }
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "due",
        lua.create_function(move |_, dt: f64| {
            with_state(&s, |state| {
                if !state.replay.enabled {
                    return Ok(false);
                }
                state.replay.accumulator += dt;
                let due = state.replay.timeline.is_empty()
                    || state.replay.accumulator >= state.replay.interval;
                if due {
                    state.replay.accumulator = 0.0;
                }
                Ok(due)
            })
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "stats",
        lua.create_function(move |lua, ()| {
            let (enabled, paused, bytes, frame, time, snapshots) = with_state(&s, |state| {
                Ok((
                    state.replay.enabled,
                    state.replay.paused,
                    state.replay.timeline.bytes(),
                    state.frame_number,
                    state.elapsed,
                    state.replay.timeline.infos(),
                ))
            })?;
            let result = lua.create_table()?;
            result.set("recording", enabled)?;
            result.set("paused", paused)?;
            result.set("bytes", bytes)?;
            result.set("frame", frame)?;
            result.set("time", time)?;
            result.set("snapshots", lua.to_value(&snapshots)?)?;
            Ok(result)
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "bookmark",
        lua.create_function(move |lua, (label, note, value): (String, String, Value)| {
            let scenes = scene_stack(lua)?;
            let registered = lua.from_value(value)?;
            with_state(&s, |state| {
                ensure!(
                    state.development_tools,
                    "bug bookmarks are disabled in this profile"
                );
                let info = kairo_core::bookmarks::BookmarkInfo {
                    id: 1,
                    label,
                    note,
                    scene: scenes.last().cloned().unwrap_or_default(),
                    frame: state.frame_number,
                    time: state.elapsed,
                    bytes: 0,
                    profile: state.profile.clone(),
                };
                let snapshot = Snapshot {
                    registered,
                    scenes,
                    input: state.input.clone(),
                    actions: state.actions.clone(),
                    physics: state.physics.snapshot()?,
                    rng: state.rng.clone(),
                    frame: state.frame_number,
                    time: state.elapsed,
                    delta: state.delta,
                };
                state
                    .replay
                    .bookmarks
                    .record(info, serde_json::to_vec(&snapshot)?)
            })
        })?,
    )?;
    let s = state.clone();
    backend.set(
        "bookmarks",
        lua.create_function(move |lua, ()| lua.to_value(&s.borrow().replay.bookmarks.infos()))?,
    )?;
    let public: Table = lua
        .load(include_str!("builtin/replay.lua"))
        .set_name("@kairo/replay.lua")
        .call(backend)?;
    lua.globals().set("replay", public)?;
    let random = lua.create_table()?;
    let s = state.clone();
    random.set(
        "seed",
        lua.create_function(move |_, seed: u64| {
            with_state(&s, |state| {
                state.rng = Rng::seeded(seed);
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    random.set(
        "float",
        lua.create_function(move |_, ()| with_state(&s, |state| Ok(state.rng.float())))?,
    )?;
    let s = state.clone();
    random.set(
        "integer",
        lua.create_function(move |_, (min, max): (i64, i64)| {
            with_state(&s, |state| state.rng.integer(min, max))
        })?,
    )?;
    lua.globals().set("random", random)
}

fn scene_stack(lua: &Lua) -> mlua::Result<Vec<String>> {
    let scene: Table = lua.globals().get("scene")?;
    let info: Table = scene.get::<mlua::Function>("info")?.call(())?;
    info.get::<Table>("stack")?
        .sequence_values::<String>()
        .collect()
}

pub(crate) fn before(lua: &Lua) -> Result<()> {
    let replay: Table = lua.globals().get("replay")?;
    replay
        .get::<mlua::Function>("_before")?
        .call::<()>(())
        .map_err(|e| anyhow::anyhow!("Kairo Replay: {e}"))
}
pub(crate) fn after(lua: &Lua, dt: f32) -> mlua::Result<()> {
    let replay: Table = lua.globals().get("replay")?;
    replay
        .get::<mlua::Function>("_after")?
        .call::<()>(dt)
        .map_err(|e| lua_error(anyhow::anyhow!("Kairo Replay: {e}")))
}

pub(crate) fn command(
    state: &SharedState,
    command: kairo_core::profiler::DebugCommand,
) -> Result<()> {
    use kairo_core::profiler::DebugCommand;
    let mut state = state.try_borrow_mut()?;
    state.replay.pending = Some(match command {
        DebugCommand::Pause => ReplayRequest::Pause,
        DebugCommand::Resume => ReplayRequest::Resume,
        DebugCommand::Step => ReplayRequest::Step,
        DebugCommand::Seek { time } => {
            ensure!(time.is_finite() && time >= 0.0, "invalid replay time");
            ReplayRequest::SeekTime(time)
        }
        DebugCommand::BookmarkJump { session, id } => {
            ensure!(
                session == state.inspection_session,
                "stale bookmark session"
            );
            state.replay.bookmarks.snapshot(id)?;
            ReplayRequest::Bookmark(id)
        }
        DebugCommand::BookmarkEdit {
            session,
            id,
            label,
            note,
        } => {
            ensure!(
                session == state.inspection_session,
                "stale bookmark session"
            );
            state.replay.bookmarks.rename(id, label, note)?;
            return Ok(());
        }
        DebugCommand::BookmarkDelete { session, id } => {
            ensure!(
                session == state.inspection_session,
                "stale bookmark session"
            );
            state.replay.bookmarks.remove(id)?;
            return Ok(());
        }
        DebugCommand::Record { enabled } => {
            state.replay.enabled = enabled;
            return Ok(());
        }
        _ => anyhow::bail!("command is not a replay command"),
    });
    Ok(())
}
