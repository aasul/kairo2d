use std::path::Path;
use std::process::Command;

#[test]
fn included_examples_pass_check_and_real_headless_execution() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "hello-world",
        "sprites",
        "movement",
        "audio",
        "physics",
        "breakout",
    ] {
        let path = root.join("examples").join(name);
        for arguments in [vec!["check"], vec!["run", "--headless", "--frames", "8"]] {
            let output = Command::new(env!("CARGO_BIN_EXE_kairo"))
                .args(arguments)
                .arg(&path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
fn invalid_game_exits_nonzero_with_a_lua_location() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("main.lua"),
        "function game.update(dt)\n error('intentional test error')\nend",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kairo"))
        .args(["run", "--headless", "--frames", "1"])
        .arg(root.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("main.lua:2"), "{error}");
    assert!(error.contains("intentional test error"), "{error}");
}

#[test]
fn cli_templates_and_host_export_are_usable_from_another_directory() {
    let root = tempfile::tempdir().unwrap();
    let binary = env!("CARGO_BIN_EXE_kairo");
    for template in ["empty", "hello-world", "movement", "breakout"] {
        let project = root.path().join(template);
        let output = Command::new(binary)
            .args(["new"])
            .arg(&project)
            .args(["--template", template, "--title", "Smoke Game"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let check = Command::new(binary)
            .args(["run", "--headless", "--frames", "4"])
            .arg(&project)
            .output()
            .unwrap();
        assert!(
            check.status.success(),
            "{}",
            String::from_utf8_lossy(&check.stderr)
        );
    }
    let project = root.path().join("breakout");
    let package = root.path().join("export");
    let output = Command::new(binary)
        .arg("build")
        .arg(project)
        .arg("--output")
        .arg(&package)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let filename = if cfg!(windows) {
        "SmokeGame.exe"
    } else {
        "SmokeGame"
    };
    let executable = package.join(filename);
    assert!(executable.is_file());
    // Exercise the copied executable and bundled game, without requiring a display.
    let output = Command::new(&executable)
        .current_dir(root.path())
        .args(["run", "--headless", "--frames", "4"])
        .arg(package.join("game"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        kairo_project::packaged_project(&executable)
            .unwrap()
            .unwrap(),
        package.join("game").canonicalize().unwrap()
    );
}
