#[cfg(feature = "gui")]
mod gui;

#[cfg(feature = "gui")]
fn main() -> anyhow::Result<()> {
    gui::run()
}

#[cfg(not(feature = "gui"))]
fn main() {
    eprintln!("water-reminder se compilo sin GUI. Usa: cargo run --release --features gui");
}
