#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

mod app;
mod drives;
mod dump;
mod postprocess;
mod theme;

// Create the application icon for OS decoration
fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/icon/icon.png");
    let img = image::load_from_memory(png_data).unwrap().to_rgba8();
    let (w, h) = img.dimensions();
    egui::IconData { rgba: img.into_raw(), width: w, height: h }
}

// Get the path to the redumper executable (first exe dir, then PATH)
pub fn find_redumper() -> Option<std::path::PathBuf> {
    let name = if cfg!(windows) { "redumper.exe" } else { "redumper" };
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(name))).filter(|p| p.exists()).or_else(|| {
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths).find_map(|dir| {
                let full = dir.join(name);
                full.exists().then_some(full)
            })
        })
    })
}

fn main() -> eframe::Result {
    // Ensure that redumper executable is available
    if find_redumper().is_none() {
        rfd::MessageDialog::new()
            .set_title("Redumper GUI")
            .set_description(
                "'redumper' executable not found.\n\nPlease place the redumper executable in the same folder as this program, or on your PATH.",
            )
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
        return Ok(());
    }

    // Prepare the application window and icons
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([480.0, 360.0])
            .with_icon(std::sync::Arc::new(load_icon()))
            .with_decorations(true),
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        dithering: false,
        ..Default::default()
    };

    // Start the application
    eframe::run_native(
        "Redumper GUI",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();

            // Use Titillium Web proportional font
            fonts.font_data.insert(
                "Titillium".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/font/titillium/TitilliumWeb-Regular.ttf"
                ))),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "Titillium".to_owned());
                family.insert(1, "Hack".to_owned());
            }

            // Use IBM Plex Mono monospaced font
            fonts.font_data.insert(
                "IBMPlexMono".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/font/ibmplexmono/IBMPlexMono-Regular.ttf"
                ))),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "IBMPlexMono".to_owned());
                family.insert(1, "Hack".to_owned());
            }

            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(app::App::new(cc)))
        }),
    )
}
