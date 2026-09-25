use crate::state::{check_options, with_state, SharedState};
use kairo_core::{SoundHandle, VoiceHandle};
use mlua::{AnyUserData, Lua, LuaSerdeExt, Table, UserData, UserDataMethods};

#[derive(Clone)]
struct Sound {
    handle: SoundHandle,
    duration: f64,
    bus: String,
}

impl UserData for Sound {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getDuration", |_, sound, ()| Ok(sound.duration));
    }
}

#[derive(Clone, Copy)]
struct Voice(VoiceHandle);
impl UserData for Voice {}

pub(crate) fn register(lua: &Lua, state: &SharedState) -> mlua::Result<()> {
    let audio = lua.create_table()?;
    let s = state.clone();
    audio.set(
        "isAvailable",
        lua.create_function(move |_, ()| with_state(&s, |s| Ok(s.audio.available())))?,
    )?;
    let s = state.clone();
    audio.set(
        "load",
        lua.create_function(move |_, (path, options): (String, Option<Table>)| {
            let bus = if let Some(options) = options {
                check_options(&options, &["bus"])?;
                options
                    .get::<Option<String>>("bus")?
                    .unwrap_or_else(|| "sfx".into())
            } else {
                "sfx".into()
            };
            with_state(&s, |s| {
                anyhow::ensure!(
                    kairo_core::mixer::BUSES.contains(&bus.as_str()),
                    "unknown audio bus"
                );
                let handle = s.audio.load(&path)?;
                Ok(Sound {
                    handle,
                    duration: s.audio.duration(handle)?,
                    bus,
                })
            })
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "play",
        lua.create_function(move |_, (sound, options): (AnyUserData, Option<Table>)| {
            let sound = sound.borrow::<Sound>()?.clone();
            let mut playback = kairo_audio::PlaybackOptions {
                bus: sound.bus,
                ..Default::default()
            };
            if let Some(options) = options {
                check_options(&options, &["volume", "looping", "bus", "pitch", "pan"])?;
                playback.volume = options.get::<Option<f32>>("volume")?.unwrap_or(1.0);
                playback.looping = options.get::<Option<bool>>("looping")?.unwrap_or(false);
                playback.bus = options
                    .get::<Option<String>>("bus")?
                    .unwrap_or(playback.bus);
                playback.pitch = options.get::<Option<f32>>("pitch")?.unwrap_or(1.0);
                playback.pan = options.get::<Option<f32>>("pan")?.unwrap_or(0.0);
            }
            with_state(&s, |s| {
                anyhow::ensure!(
                    !s.loading,
                    "start playback from update/input, not while loading a candidate session"
                );
                Ok(Voice(s.audio.play_with(sound.handle, &playback)?))
            })
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "stop",
        lua.create_function(move |_, voice: AnyUserData| {
            let voice = voice.borrow::<Voice>()?.0;
            with_state(&s, |s| {
                s.audio.stop(voice);
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "stopAll",
        lua.create_function(move |_, ()| {
            with_state(&s, |s| {
                s.audio.stop_all();
                Ok(())
            })
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "setVolume",
        lua.create_function(move |_, (voice, volume): (AnyUserData, f32)| {
            let voice = voice.borrow::<Voice>()?.0;
            with_state(&s, |s| s.audio.set_volume(voice, volume))
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "setMasterVolume",
        lua.create_function(move |_, volume: f32| {
            with_state(&s, |s| s.audio.set_master_volume(volume))
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "isPlaying",
        lua.create_function(move |_, voice: AnyUserData| {
            let voice = voice.borrow::<Voice>()?.0;
            with_state(&s, |s| Ok(s.audio.is_playing(voice)))
        })?,
    )?;

    audio.set("loadSound", audio.get::<mlua::Function>("load")?)?;
    let s = state.clone();
    audio.set(
        "setBusVolume",
        lua.create_function(
            move |_, (bus, volume, seconds): (String, f32, Option<f64>)| {
                with_state(&s, |state| {
                    state
                        .audio
                        .set_bus(&bus, Some(volume), None, seconds.unwrap_or(0.0))
                })
            },
        )?,
    )?;
    let s = state.clone();
    audio.set(
        "muteBus",
        lua.create_function(move |_, (bus, muted): (String, bool)| {
            with_state(&s, |state| {
                state.audio.set_bus(&bus, None, Some(muted), 0.0)
            })
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "stopBus",
        lua.create_function(move |_, bus: String| {
            with_state(&s, |state| state.audio.stop_bus(&bus))
        })?,
    )?;
    let s = state.clone();
    audio.set(
        "mixer",
        lua.create_function(move |lua, ()| {
            let values = with_state(&s, |state| Ok(state.audio.mixer()))?;
            lua.to_value(&values)
        })?,
    )?;
    lua.globals().set("audio", audio)
}
