use anyhow::{Context, Result};
use mj_universel::{storage::Storage, ui::GameApp};

fn main() -> Result<()> {
    let storage = Storage::open_default().context("Impossible d'ouvrir la base du jeu")?;
    let app = GameApp::new(storage).context("Impossible de charger les campagnes")?;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 740.0])
            .with_min_inner_size([760.0, 540.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MJ universel — L2 finale",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| anyhow::anyhow!("Erreur d'interface: {err}"))
}
