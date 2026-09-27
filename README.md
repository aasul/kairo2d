# Kairo2D 3.4.0

I'm building Kairo2D as a small 2D game engine written in Rust, with Lua 5.4 for game
code and Kairo as its desktop editor. It includes a scene system, input actions, an
inspector, particles, animation state machines, audio buses, tilemaps, and a few
tools for working with a running game.

**Project status:** Kairo2D is experimental. This 3.4.0 development checkout builds
and passes native tests on the local Windows host, but has not completed visual or
cross-platform release acceptance. APIs may change. Contributions and careful bug reports are
welcome. See [VALIDATION.md](VALIDATION.md) for what was checked and what still needs
to be tested.

## What is in the project

- **Kairo editor:** edit Lua and project files, make pixel art and animation data,
  and run games from one desktop app.
- **Scenes and input actions:** organize game states, author validated node hierarchies
  in `.scene` files, and map keyboard, mouse, or controller input to named actions and axes.
- **Live Inspector and Kairo Link:** inspect explicitly exposed game values locally
  or during a trusted remote development session.
- **Kairo Replay:** record bounded snapshots and save bookmarks while debugging.
- **Kairo Micro:** draw to a small virtual canvas with a fixed palette.
- **Game systems:** particles, animation state machines, audio mixer buses, Tiled
  tilemaps, physics, saves, and localization.
- **Standalone packaging:** prepare a game folder with the runtime built for the
  current host platform.

These features are still experimental. Link, Replay, remote inspection, and native
extensions need more testing. [Known limitations](docs/known-limitations.md) lists
what is missing and where current behavior stops.

## Get started

You will need a stable Rust toolchain with Cargo, rustfmt, and Clippy; a native C/C++
toolchain for the bundled Lua library; and Python 3.11 or newer for repository checks
and packaging. Linux also needs the development libraries listed in
[the CI workflow](.github/workflows/ci.yml). Windows builds use the Visual Studio C++
tools and Windows SDK. macOS builds need Apple's developer tools.

From the repository directory, build and open the editor:

```sh
cargo fmt --all
cargo build --workspace -j 1
cargo run -p kairo-editor -- examples/top-down
```

To make and run a new project:

```sh
cargo run -p kairo-cli -- new mygame --template top-down --title "My game"
cargo run -p kairo-cli -- check mygame
cargo run -p kairo-cli -- run mygame --profile development
```

The editor and game runtime are separate programs. When packaging on Windows, both
`Kairo.exe` and `bin/kairo.exe` need to be present. See [Getting started](docs/getting-started.md)
and [the editor guide](docs/editor.md) for more detail.

## Examples

The `examples` directory contains small projects for scenes, input, animation,
particles, audio, tilemaps, physics, saves, and the development tools. Start with
`examples/top-down` for a game that uses several systems together. Each example has
its own README with controls and run instructions. The original included art and
sound assets are MIT licensed; no fonts are bundled.

## Checks and release status

The repository includes Rust tests, Lua checks, static source checks, and CI for
Windows, Linux, and macOS. The tests in the source tree have not all been run against
a native build for this 3.4.0 snapshot. A successful static or Lua check does not
confirm rendering, audio, controller, editor, or network behavior. See
[VALIDATION.md](VALIDATION.md) for the recorded results and
[release acceptance](docs/release-acceptance.md) for the remaining checks.

## Contributing

Contributions are welcome. Please open an issue for a bug or a clearly scoped change,
then send a focused pull request with the checks you ran and anything you could not
verify. Start with [CONTRIBUTING.md](CONTRIBUTING.md). Security issues should be
reported privately using the instructions in [SECURITY.md](SECURITY.md).

The engine, editor, and original examples are distributed under the
[MIT license](LICENSE). Third-party notices are in [THIRD_PARTY.md](THIRD_PARTY.md).
