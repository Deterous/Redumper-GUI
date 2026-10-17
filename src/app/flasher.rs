use eframe::egui;

use super::App;
use crate::drives;
use crate::dump;
use crate::theme::ColorTheme;

struct FlashType {
    label: &'static str,
    command: &'static str,
    help: &'static str,
}

const FLASH_TYPES: &[FlashType] = &[
    FlashType {
        label: "MT1959",
        command: "flash::mt1959",
        help: "Flash LG/ASUS blu-ray drives with MT1959 chipset (including OmniDrive).",
    },
    FlashType {
        label: "MT1339",
        command: "flash::mt1339",
        help: "Flash Toshiba/Samsung DVD drives with MT1339 chipset (including Kreon).",
    },
    FlashType { label: "SD616", command: "flash::sd616", help: "Flash Samsung SD-616F/T drives." },
    FlashType { label: "Plextor", command: "flash::plextor", help: "Flash Plextor CD/DVD drives with Sanyo chipsets." },
];

impl App {
    pub(super) fn paint_flasher_modal(&mut self, ui: &mut egui::Ui, t: &ColorTheme, is_running: bool) {
        if !self.show_flasher {
            return;
        }

        let ctx = ui.ctx().clone();
        let screen = ctx.input(|i| i.viewport_rect());

        // Dim the main app
        egui::Area::new(egui::Id::new("flasher_overlay")).fixed_pos(screen.min).order(egui::Order::Middle).show(
            &ctx,
            |ui| {
                ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha(160));
                ui.allocate_rect(screen, egui::Sense::click_and_drag());
            },
        );

        let fp = *self.dump.flash_progress.lock().unwrap();
        let flashing = is_running && fp.is_some();

        // Modal window on top of the overlay
        egui::Window::new("flasher_modal")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([420.0, 0.0])
            .order(egui::Order::Foreground)
            .frame(
                egui::Frame::new()
                    .fill(t.bg_panel)
                    .stroke(egui::Stroke::new(1.0, t.accent))
                    .inner_margin(16.0),
            )
            .show(&ctx, |ui| {
                // Title row with close button
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("DRIVE FLASHER").color(t.highlight).size(14.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 22.0), egui::Sense::click());
                        let color = if resp.hovered() && !flashing {
                            ui.painter().rect_filled(rect, 3.0, t.accent_stop);
                            egui::Color32::WHITE
                        } else {
                            t.text_dim
                        };
                        if resp.clicked() && !flashing {
                            self.show_flasher = false;
                            *self.dump.flash_progress.lock().unwrap() = None;
                            self.flasher_firmware.clear();
                        }
                        let c = rect.center();
                        let s = 4.5;
                        ui.painter().line_segment(
                            [egui::pos2(c.x - s, c.y - s), egui::pos2(c.x + s, c.y + s)],
                            egui::Stroke::new(1.5, color),
                        );
                        ui.painter().line_segment(
                            [egui::pos2(c.x + s, c.y - s), egui::pos2(c.x - s, c.y + s)],
                            egui::Stroke::new(1.5, color),
                        );
                    });
                });

                ui.add_space(12.0);

                if flashing {
                    // While flashing
                    ui.label(egui::RichText::new("DO NOT UNPLUG THE DRIVE").color(egui::Color32::RED).size(13.0).strong());
                    ui.add_space(4.0);
                    Self::paint_flasher_progress(ui, t, fp.unwrap_or(0.0));
                    ui.add_space(4.0);
                    let log = self.dump.log.lock().unwrap_or_else(|e| e.into_inner());
                    let last_line = log.lines().rev().find(|l| { let t = l.trim_start(); !t.is_empty() && !t.starts_with("[Redumper") && !t.starts_with("[Failed") }).unwrap_or("");
                    ui.label(
                        egui::RichText::new(last_line)
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_dim),
                    );
                } else {
                    // When not flashing, show all options (and progress bar if a flash has run)
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("DRIVE TYPE ▸").color(t.accent).size(13.0));
                        egui::ComboBox::from_id_salt("flasher_command")
                            .width(120.0)
                            .selected_text(
                                egui::RichText::new(FLASH_TYPES[self.flasher_command].label)
                                    .monospace()
                                    .size(13.0)
                                    .color(t.highlight),
                            )
                            .show_ui(ui, |ui| {
                                for (i, ft) in FLASH_TYPES.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.flasher_command,
                                        i,
                                        egui::RichText::new(ft.label).monospace().size(13.0).color(t.highlight),
                                    );
                                }
                            });
                    });

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(FLASH_TYPES[self.flasher_command].help)
                            .color(t.text_dim)
                            .size(12.0),
                    );

                    ui.add_space(10.0);

                    // Drive selector
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("DRIVE ▸").color(t.accent).size(13.0));
                        let drive_text = self.drives.get(self.flasher_drive).map(|s| s.0.as_str()).unwrap_or("(none)");
                        egui::ComboBox::from_id_salt("flasher_drive")
                            .width(80.0)
                            .selected_text(
                                egui::RichText::new(drive_text).monospace().size(13.0).color(t.highlight),
                            )
                            .show_ui(ui, |ui| {
                                for (i, (d, _)) in self.drives.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.flasher_drive,
                                        i,
                                        egui::RichText::new(d.as_str()).monospace().size(13.0).color(t.highlight),
                                    );
                                }
                            });

                        // Refresh drive button
                        let refresh_ready = self.last_drive_refresh.elapsed() >= std::time::Duration::from_secs(2);
                        ui.add_enabled_ui(refresh_ready, |ui| {
                            if ui.button(egui::RichText::new("🔃").color(t.highlight)).on_hover_text("Refresh drives").clicked() {
                                self.last_drive_refresh = std::time::Instant::now();
                                let prev = self.drives.get(self.flasher_drive).cloned();
                                self.drives = drives::detect_drives();
                                self.flasher_drive = prev
                                    .and_then(|p| self.drives.iter().position(|d| d.0 == p.0))
                                    .unwrap_or(0);
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Firmware file picker
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("FIRMWARE ▸").color(t.accent).size(13.0));
                        if ui.button(egui::RichText::new("BROWSE").color(t.highlight).size(13.0)).clicked() {
                            if let Some(path) = rfd::FileDialog::new().set_title("Select firmware file").add_filter("Firmware", &["bin"]).add_filter("All files", &["*"]).pick_file() {
                                self.flasher_firmware = path.to_string_lossy().to_string();
                            }
                        }
                    });
                    if !self.flasher_firmware.is_empty() {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(&self.flasher_firmware).monospace().color(t.text_dim).size(12.0),
                        );
                    }

                    ui.add_space(14.0);

                    // Flash button
                    let can_flash = !self.drives.is_empty() && !self.flasher_firmware.is_empty();
                    ui.add_enabled_ui(can_flash, |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("FLASH FIRMWARE").color(egui::Color32::BLACK).size(13.0),
                                )
                                .fill(t.accent_action),
                            )
                            .clicked()
                        {
                            let drive_name = self
                                .drives
                                .get(self.flasher_drive)
                                .map(|d| d.0.as_str())
                                .unwrap_or("(unknown)");
                            let confirmed = rfd::MessageDialog::new()
                                .set_title("Confirm Firmware Flash")
                                .set_description(&format!(
                                    "You are about to flash this drive: {}\n\nEnsure the drive tray is empty and closed.\n\nAre you sure you want to flash?",
                                    drive_name
                                ))
                                .set_level(rfd::MessageLevel::Warning)
                                .set_buttons(rfd::MessageButtons::YesNo)
                                .show()
                                == rfd::MessageDialogResult::Yes;

                            if confirmed {
                                let ft = &FLASH_TYPES[self.flasher_command];
                                let mut args = vec![ft.command.to_string()];
                                if let Some((drive, _)) = self.drives.get(self.flasher_drive) {
                                    args.push(format!("--drive={}", drive));
                                }
                                args.push(format!("--firmware={}", self.flasher_firmware));
                                if let Ok(mut log) = self.dump.log.lock() {
                                    log.clear();
                                }
                                *self.dump.flash_progress.lock().unwrap() = Some(0.0);
                                self.action_pending = true;
                                let work_dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
                                dump::run_redumper(
                                    args,
                                    None,
                                    work_dir,
                                    false,
                                    None,
                                    self.dump.clone(),
                                    ui.ctx().clone(),
                                );
                            }
                        }
                    });

                    // Progress bar and last log line (shown after a flash has been started)
                    if let Some(progress) = fp {
                        ui.add_space(10.0);
                        Self::paint_flasher_progress(ui, t, progress);
                        ui.add_space(4.0);
                        let log = self.dump.log.lock().unwrap_or_else(|e| e.into_inner());
                        let last_line = log.lines().rev().find(|l| { let t = l.trim_start(); !t.is_empty() && !t.starts_with("[Redumper") && !t.starts_with("[Failed") }).unwrap_or("");
                        ui.label(
                            egui::RichText::new(last_line)
                                .font(egui::FontId::monospace(12.0))
                                .color(t.text_dim),
                        );
                    }
                }
            });
    }

    fn paint_flasher_progress(ui: &mut egui::Ui, t: &ColorTheme, progress: f32) {
        let desired_size = egui::vec2(ui.available_width(), 18.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 2.0, t.log_bg);
        let fill_width = (rect.width() * progress.clamp(0.0, 1.0)).max(0.0);
        if fill_width > 0.0 {
            let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_width, rect.height()));
            painter.rect_filled(fill_rect, 2.0, t.accent_action);
        }
        painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, t.border), egui::StrokeKind::Middle);
        let label = if progress >= 1.0 { "Done!".to_string() } else { format!("{}%", (progress * 100.0) as u8) };
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), t.highlight);
    }
}
