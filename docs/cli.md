# CLI

The runtime executable is `kairo` (`kairo.exe` on Windows). The desktop executable is
`kairo-editor` during development and `Kairo.exe` in a Windows distribution.

```sh
kairo --version
kairo new mygame --template hello-world --title "My game"
kairo check mygame
kairo doctor mygame
kairo run mygame
kairo build mygame --output exports/MyGame
```

`new` requires a new child directory of an existing parent and refuses overwrites.
Templates: `empty`, `hello-world`, `movement`, `breakout`, `micro`, `ui`, `link`,
`replay`, `scenes`, `input-actions`, `live-inspector`, `particles`, `audio-mixer`,
`replay-bugs`, `top-down`, `platformer`. It creates assets/, writes the selected files and assigns a save identity.

`check` parses config and compiles all project `.lua` sources as Lua 5.4 text. It
opens no devices and does not execute game code, infer dynamic asset paths or prove
that all API calls are valid. `doctor` additionally prints the runtime version,
executable location and canonical project path.

`run` defaults to the current directory. Useful options:

| Option | Meaning |
| --- | --- |
| `--profile NAME` | Runtime profile: development (default), debug, release or micro. |
| `--no-audio` | Do not open an output device. |
| `--no-watch` | Disable local automatic reload and fail on game errors; F5 restarts manually. |
| `--headless` | Execute Lua/physics/assets/CPU geometry without a window or GPU. |
| `--frames N` | Exit after N frames; default headless count is 120. |
| `--link HOST:PORT` | Explicitly connect to a trusted development host. |
| `--link-token-file PATH` | Private token file; required together with --link. |

Link currently requires windowed execution. `--controlled` is an internal editor
stdin/telemetry channel, not a public network interface. stdin closure terminates
controlled runs.

`build` validates Lua/config and copies the current host runtime with the game into
a **new** directory. Default output is `PROJECT/dist/game`; `--output` can select an
external directory. Arbitrary other in-project output paths and existing outputs
are rejected. This is packaging, not Rust cross-compilation. `build --profile NAME` defaults to
`release` and records it in the package marker. A no-argument packaged launch uses
that profile. Changing a runtime profile does not change compiler optimization.
The editor exports its currently selected profile, so choose Release explicitly.
See [build profiles](build-profiles.md).

```sh
kairo link-host examples/link --bind 127.0.0.1:7743 --token-file ../session.kairo-token
kairo run examples/link --link 127.0.0.1:7743 --link-token-file ../session.kairo-token
```

Use separate terminals. The token file must be new and outside the shared project.
Stop CLI hosting with the terminal interrupt (Ctrl+C). See [Link](kairo-link.md).

When invoking through Cargo, add the crate selector and separator:
`cargo run -p kairo-cli -- run examples/micro`.
