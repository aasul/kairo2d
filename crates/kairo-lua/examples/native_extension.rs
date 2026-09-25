use anyhow::Result;
use kairo_core::{
    extensions::{Canvas, NativeExtension},
    Color, Config, InputState, ProjectFs,
};
use kairo_lua::GameSession;

struct Marker {
    x: f32,
}
impl NativeExtension for Marker {
    fn name(&self) -> &str {
        "moving-marker"
    }
    fn update(&mut self, dt: f32, _input: &InputState) -> Result<()> {
        self.x += dt * 40.0;
        Ok(())
    }
    fn draw(&mut self, canvas: &mut Canvas<'_>) -> Result<()> {
        canvas.rectangle([self.x, 100.0], [16.0, 16.0], Color::WHITE)
    }
}
fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "examples/hello-world".into());
    let fs = ProjectFs::new(path)?;
    let config = Config::load(&fs)?;
    let session = GameSession::load(fs, &config, false)?;
    session.add_extension(Marker { x: 20.0 })?;
    for _ in 0..60 {
        session.tick(1.0 / 60.0)?;
    }
    println!(
        "Native extension generated {} scene commands",
        session.state().borrow().frame.commands.len()
    );
    Ok(())
}
