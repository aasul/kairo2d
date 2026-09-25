use kairo_core::ProjectFs;
use mlua::{ChunkMode, Function, Lua, Table, Value};

pub(crate) fn install(lua: &Lua, fs: ProjectFs) -> mlua::Result<()> {
    let globals = lua.globals();
    for name in ["dofile", "loadfile"] {
        globals.set(name, Value::Nil)?;
    }
    let package: Table = globals.get("package")?;
    package.set("loadlib", Value::Nil)?;
    package.set("cpath", "")?;
    package.set("path", "")?;
    let old: Table = package.get("searchers")?;
    let searchers = lua.create_table()?;
    searchers.set(1, old.get::<Function>(1)?)?;
    searchers.set(
        2,
        lua.create_function(move |lua, name: String| {
            if !valid_name(&name) {
                return Err(mlua::Error::RuntimeError(format!(
                    "invalid module name '{name}'; use dotted identifiers"
                )));
            }
            let stem = name.replace('.', "/");
            for path in [format!("{stem}.lua"), format!("{stem}/init.lua")] {
                if fs.exists(&path).map_err(crate::state::lua_error)? {
                    let source = fs.read_text(&path).map_err(crate::state::lua_error)?;
                    let function = lua
                        .load(&source)
                        .set_name(format!("@{path}"))
                        .set_mode(ChunkMode::Text)
                        .into_function()?;
                    return Ok(Value::Function(function));
                }
            }
            Ok(Value::String(lua.create_string(format!(
                "\n\tno project module '{name}' ({stem}.lua or {stem}/init.lua)"
            ))?))
        })?,
    )?;
    package.set("searchers", searchers)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_names_cannot_be_paths() {
        assert!(valid_name("game.player_2"));
        for name in [
            "",
            "../secret",
            "a..b",
            "/etc/passwd",
            "a/b",
            "C:drive",
            "a\\b",
        ] {
            assert!(!valid_name(name), "{name}");
        }
    }
}
