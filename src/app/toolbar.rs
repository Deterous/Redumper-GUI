use eframe::egui;

use super::App;
use crate::drives;
use crate::dump;
use crate::theme::ColorTheme;

impl App {
    // Paint a beveled toggle button with raised/sunken states
    fn paint_beveled_button(
        ui: &egui::Ui,
        rect: egui::Rect,
        pressed: bool,
        label: &str,
        fill: egui::Color32,
        font_size: f32,
        text_color: egui::Color32,
    ) {
        let painter = ui.painter();

        // Dim background color when pressed
        let base = if pressed {
            egui::Color32::from_rgb(
                (fill.r() as u16 * 7 / 10) as u8,
                (fill.g() as u16 * 7 / 10) as u8,
                (fill.b() as u16 * 7 / 10) as u8,
            )
        } else {
            fill
        };
        painter.rect_filled(rect, 2.0, base);

        // Create bevel effect
        let (tl, br) = if pressed {
            (egui::Color32::from_rgb(10, 10, 10), egui::Color32::from_rgb(150, 150, 150))
        } else {
            (egui::Color32::from_rgb(200, 200, 200), egui::Color32::from_rgb(15, 15, 15))
        };
        painter.line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.5, tl));
        painter.line_segment([rect.left_top(), rect.left_bottom()], egui::Stroke::new(1.5, tl));
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], egui::Stroke::new(1.5, br));
        painter.line_segment([rect.right_top(), rect.right_bottom()], egui::Stroke::new(1.5, br));

        // Paint text offset if pressed
        let offset = if pressed { egui::vec2(1.0, 1.0) } else { egui::Vec2::ZERO };
        painter.text(
            rect.center() + offset,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(font_size),
            text_color,
        );
    }

    // Calculate size of a beveled button based on a given string
    fn beveled_button_size(label: &str, height: f32, font_size: f32, padding: f32) -> egui::Vec2 {
        let width = label.chars().count() as f32 * font_size * 0.6 + padding * 2.0;
        egui::vec2(width, height)
    }

    // Paint the toolbar panel
    pub(super) fn paint_toolbar(&mut self, ui: &mut egui::Ui, t: &ColorTheme, is_running: bool) {
        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(t.bg_panel)
                    .inner_margin(egui::Margin::same(10))
                    .stroke(egui::Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                // Disable drive/path toolbar when dump is running
                ui.add_enabled_ui(!is_running, |ui| {
                    // First row
                    ui.horizontal(|ui| {
                        // Drive selector
                        ui.label(egui::RichText::new("DRIVE ▸").color(t.accent).size(13.0));
                        let drive_text = self.drives.get(self.selected_drive).map(|s| s.0.as_str()).unwrap_or("(none)");
                        egui::ComboBox::from_id_salt("drive")
                            .width(80.0)
                            .selected_text(egui::RichText::new(drive_text).monospace().size(13.0).color(t.highlight))
                            .show_ui(ui, |ui| {
                                for (i, (d, _)) in self.drives.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.selected_drive,
                                        i,
                                        egui::RichText::new(d.as_str()).monospace().size(13.0).color(t.highlight),
                                    );
                                }
                            });

                        // Refresh drive button
                        let refresh_ready = self.last_drive_refresh.elapsed() >= std::time::Duration::from_secs(2);
                        ui.add_enabled_ui(refresh_ready, |ui| {
                            if ui.button(egui::RichText::new("🔃").color(t.highlight)).on_hover_text("Refresh drives").clicked()
                            {
                                self.last_drive_refresh = std::time::Instant::now();
                                let prev = self.drives.get(self.selected_drive).cloned();
                                self.drives = drives::detect_drives();
                                if let Some(prev) = &prev
                                    && let Some(idx) = self.drives.iter().position(|d| d.0 == prev.0)
                                {
                                    self.selected_drive = idx;
                                    self.prev_selected_drive = idx;
                                    if self.drives[idx].1 != prev.1 {
                                        self.disc_name.clear();
                                    }
                                } else {
                                    self.selected_drive = 0;
                                    self.prev_selected_drive = usize::MAX;
                                    self.disc_name.clear();
                                }
                            }
                        });

                        ui.add_space(4.0);

                        // If volume identifier is found, show it next to the selected drive
                        let volume_label = self.drives.get(self.selected_drive).map(|s| s.1.as_str()).unwrap_or("");
                        ui.label(egui::RichText::new(volume_label).monospace().color(t.text_dim).size(13.0));
                    });

                    ui.add_space(4.0);

                    // Second row
                    ui.horizontal(|ui| {
                        // Base output path selector
                        ui.label(egui::RichText::new("PATH ▸").color(t.accent).size(13.0));
                        if ui.button(egui::RichText::new("BROWSE").color(t.highlight).size(13.0)).clicked()
                            && let Some(path) =
                                rfd::FileDialog::new().set_title("Choose base output folder").pick_folder()
                        {
                            self.output_dir = Some(path);
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("NAME ▸").color(t.accent).size(13.0));
                        ui.scope(|ui| {
                            // Highlight disc name box if it is empty
                            if self.disc_name.is_empty() {
                                let bg = ui.visuals().extreme_bg_color;
                                ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(
                                    ((bg.r() as u16 * 3 + t.accent_stop.r() as u16) / 4) as u8,
                                    ((bg.g() as u16 * 3 + t.accent_stop.g() as u16) / 4) as u8,
                                    ((bg.b() as u16 * 3 + t.accent_stop.b() as u16) / 4) as u8,
                                );
                                ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(2.0, t.accent_stop);
                                ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::new(2.0, t.accent_stop);
                                ui.visuals_mut().selection.stroke = egui::Stroke::new(2.0, t.accent_stop);
                            }

                            // Disc name input box
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.disc_name)
                                    .desired_width(200.0)
                                    .font(egui::TextStyle::Monospace)
                                    .text_color(t.highlight),
                            );

                            // Limit disc name to 100 characters and disallow illegal characters
                            if resp.changed() {
                                self.disc_name
                                    .retain(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'));
                                if self.disc_name.chars().count() > 100 {
                                    self.disc_name = self.disc_name.chars().take(100).collect();
                                }
                            }
                        });
                    });

                    // Show selected output path, warn if dump already exists
                    let dir = self.effective_output_dir();
                    let display = dir.display().to_string();
                    if self.dump_exists {
                        let msg = if self.complete_dump_exists {
                            format!("Complete dump exists at {}", display)
                        } else {
                            format!("Partial dump exists at {}", display)
                        };
                        ui.label(egui::RichText::new(msg).monospace().color(t.accent_stop).size(13.0));
                    } else {
                        ui.label(egui::RichText::new(display).monospace().color(t.text_dim).size(13.0));
                    }
                });

                ui.add_space(6.0);

                // Action buttons
                ui.horizontal(|ui| {
                    let can_start =
                        !is_running && !self.drives.is_empty() && !self.dump_exists && !self.disc_name.is_empty();
                    let (btn_h, btn_font, btn_pad) = (22.0, 13.0, 6.0); // Button height, font size, and padding

                    ui.add_space(4.0);

                    // Only enable dump button if dump is able to start
                    ui.add_enabled_ui(can_start, |ui| {
                        let dump_label = "▶ DUMP";
                        let size = Self::beveled_button_size(dump_label, btn_h, btn_font, btn_pad);
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                        if response.clicked() {
                            let mut proceed = true;
                            let dir = self.effective_output_dir();
                            // Check for low disk space before dumping, warn user if less than 1GB
                            if let Ok(available) = fs2::available_space(&dir.ancestors().find(|p| p.exists()).unwrap_or(dir.as_path())) {
                                if available < 1073741824 {
                                    proceed = rfd::MessageDialog::new()
                                        .set_title("Low Disk Space")
                                        .set_description(&format!(
                                            "Only {} MB free on the output drive.\nA disc dump may require several GB.\n\nContinue anyway?",
                                            available / 1048576
                                        ))
                                        .set_level(rfd::MessageLevel::Warning)
                                        .set_buttons(rfd::MessageButtons::YesNo)
                                        .show() == rfd::MessageDialogResult::Yes;
                                }
                            }
                            if proceed {
                                // Call redumper as subprocess with output path as the working directory
                                if let Ok(mut log) = self.dump.log.lock() {
                                    log.clear();
                                }
                                self.dump.clear_sectors();
                                self.box_colors.clear();
                                self.disc.profile_texture = None;
                                self.dump_just_finished = false;
                                self.show_options = false;
                                self.action_pending = true;
                                dump::run_redumper(
                                    self.build_args(false),
                                    Some(self.output_path()),
                                    self.effective_output_dir(),
                                    self.cleanup,
                                    self.dump.clone(),
                                    ui.ctx().clone(),
                                );
                            }
                        }
                        Self::paint_beveled_button(
                            ui,
                            rect,
                            !can_start || self.action_pending || response.is_pointer_button_down_on(),
                            dump_label,
                            t.accent_action,
                            btn_font,
                            egui::Color32::BLACK,
                        );
                    });

                    // Only enable abort button if dump is running
                    ui.add_enabled_ui(is_running, |ui| {
                        let abort_label = "⏹ ABORT";
                        let size = Self::beveled_button_size(abort_label, btn_h, btn_font, btn_pad);
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                        if response.clicked()
                            && let Ok(mut guard) = self.dump.child.lock()
                            && let Some(child) = guard.take()
                        {
                            self.action_pending = true;
                            dump::graceful_kill_async(child);
                            *self.dump.hash_progress.lock().unwrap() = None;
                        }
                        let abort_text =
                            if t.accent_stop.r() < 100 && t.accent_stop.g() < 100 && t.accent_stop.b() < 100 {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::BLACK
                            };
                        Self::paint_beveled_button(
                            ui,
                            rect,
                            !is_running || self.action_pending || response.is_pointer_button_down_on(),
                            abort_label,
                            t.accent_stop,
                            btn_font,
                            abort_text,
                        );
                    });

                    // Only enable refine button if partial refinable dump exists at given path
                    let can_refine = !is_running && !self.drives.is_empty() && self.dump_refinable;
                    ui.add_enabled_ui(can_refine, |ui| {
                        let refine_label = "🔁 REFINE";
                        let size = Self::beveled_button_size(refine_label, btn_h, btn_font, btn_pad);
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                        if response.clicked() {
                            // Call redumper as subprocess with output path as the working directory
                            if let Ok(mut log) = self.dump.log.lock() {
                                log.clear();
                            }
                            self.dump_just_finished = false;
                            self.show_options = false;
                            self.action_pending = true;
                            dump::run_redumper(
                                self.build_args(true),
                                Some(self.output_path()),
                                self.effective_output_dir(),
                                self.cleanup,
                                self.dump.clone(),
                                ui.ctx().clone(),
                            );
                        }
                        Self::paint_beveled_button(
                            ui,
                            rect,
                            !can_refine || self.action_pending || response.is_pointer_button_down_on(),
                            refine_label,
                            t.accent_action,
                            btn_font,
                            egui::Color32::BLACK,
                        );
                    });

                    ui.add_enabled_ui(!is_running, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(4.0);
                            let label = if self.show_options { "▲ OPTIONS" } else { "▼ OPTIONS" };
                            let size = Self::beveled_button_size(label, btn_h, btn_font, btn_pad);
                            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                            if response.clicked() {
                                self.show_options = !self.show_options;
                            }
                            let fill = ui.visuals().widgets.inactive.bg_fill;
                            Self::paint_beveled_button(ui, rect, self.show_options, label, fill, btn_font, t.highlight);

                            ui.add_space(4.0);
                            ui.add_enabled_ui(self.dump_exists, |ui| {
                                let view_dump_label = "⎗ VIEW DUMP";
                                let size = Self::beveled_button_size(view_dump_label, btn_h, btn_font, btn_pad);
                                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                                if response.clicked() {
                                    let dir = self.effective_output_dir();
                                    if dir.exists() {
                                        open::that(&dir).ok();
                                    }
                                }
                                let fill = ui.visuals().widgets.inactive.bg_fill;
                                Self::paint_beveled_button(
                                    ui,
                                    rect,
                                    !self.dump_exists || response.is_pointer_button_down_on(),
                                    view_dump_label,
                                    fill,
                                    btn_font,
                                    t.highlight,
                                );
                            });
                        });
                    });
                });

                if self.show_options {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.separator();
                        let speed_hint = "Requested drive speed (drive may not support it)";
                        ui.label(egui::RichText::new("SPEED ▸").color(t.accent).size(13.0))
                            .on_hover_text(speed_hint);
                        ui.scope(|ui| {
                            ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                            ui.style_mut().override_font_id = Some(egui::FontId::proportional(13.0));
                            ui.visuals_mut().override_text_color = Some(t.highlight);
                            ui.spacing_mut().slider_width = 100.0;
                            const SPEED_STEPS: [i32; 14] = [1, 2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 48, 52, 100];
                            let mut step_idx = SPEED_STEPS
                                .iter()
                                .position(|&s| s == self.speed)
                                .unwrap_or_else(|| SPEED_STEPS.iter().rposition(|&s| s <= self.speed.max(1)).unwrap_or(0));
                            let slider_resp = ui.add(
                                egui::Slider::new(&mut step_idx, 0..=(SPEED_STEPS.len() - 1))
                                    .show_value(false)
                                    .custom_formatter(|v, _| {
                                        let s = SPEED_STEPS[v as usize];
                                        if s == 100 {
                                            "MAX".to_string()
                                        } else {
                                            format!("{}x", s)
                                        }
                                    }),
                            ).on_hover_text(speed_hint);
                            if slider_resp.changed() {
                                self.speed = SPEED_STEPS[step_idx];
                            }

                            let mut speed_text = if self.speed == 100 {
                                "MAX".to_string()
                            } else {
                                format!("{}x", self.speed)
                            };
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut speed_text)
                                    .desired_width(40.0)
                                    .font(egui::TextStyle::Monospace),
                            ).on_hover_text(speed_hint);
                            if resp.changed() {
                                let trimmed = speed_text.trim().trim_end_matches('x').trim().to_uppercase();
                                if trimmed == "MAX" {
                                    self.speed = 100;
                                } else if let Ok(v) = trimmed.parse::<i32>() {
                                    self.speed = v.clamp(0, 72);
                                }
                            }
                        });

                        // Retries option selector
                        ui.add_space(12.0);
                        ui.separator();
                        let refines_hint = "Number of times a bad sector will be re-read before giving up";
                        ui.label(egui::RichText::new("REFINES ▸").color(t.accent).size(13.0))
                            .on_hover_text(refines_hint);
                        ui.add(egui::DragValue::new(&mut self.retries).range(0..=u32::MAX).speed(0.1))
                            .on_hover_text(refines_hint);

                        // Auto eject option checkbox
                        ui.add_space(12.0);
                        ui.separator();
                        let auto_eject_hint = "Eject the disc as soon as redumper does not need it (redumper will continue to hash afterwards)";
                        if ui.add(egui::Label::new(egui::RichText::new("AUTO-EJECT ▸").color(t.accent).size(13.0)).sense(egui::Sense::click()))
                            .on_hover_text(auto_eject_hint)
                            .clicked()
                        {
                            self.auto_eject = !self.auto_eject;
                        }
                        ui.checkbox(&mut self.auto_eject, "")
                            .on_hover_text(auto_eject_hint);

                        // Skeleton option checkbox
                        ui.add_space(12.0);
                        ui.separator();
                        let skeleton_hint = "Hash the files within the dump and produce a filesystem skeleton";
                        if ui.add(egui::Label::new(egui::RichText::new("SKELETON ▸").color(t.accent).size(13.0)).sense(egui::Sense::click()))
                            .on_hover_text(skeleton_hint)
                            .clicked()
                        {
                            self.skeleton = !self.skeleton;
                        }
                        ui.checkbox(&mut self.skeleton, "")
                            .on_hover_text(skeleton_hint);

                        // Rings option checkbox
                        ui.add_space(12.0);
                        ui.separator();
                        let rings_hint = "Select this option when dumping discs with rings such as Dreamcast or Datel discs";
                        if ui.add(egui::Label::new(egui::RichText::new("RINGS ▸").color(t.accent).size(13.0)).sense(egui::Sense::click()))
                            .on_hover_text(rings_hint)
                            .clicked()
                        {
                            self.rings = !self.rings;
                        }
                        ui.checkbox(&mut self.rings, "")
                            .on_hover_text(rings_hint);

                        // Cleanup option checkbox
                        ui.add_space(12.0);
                        ui.separator();
                        let cleanup_hint = "Zip log files and delete intermediate files";
                        if ui.add(egui::Label::new(egui::RichText::new("CLEANUP ▸").color(t.accent).size(13.0)).sense(egui::Sense::click()))
                            .on_hover_text(cleanup_hint)
                            .clicked()
                        {
                            self.cleanup = !self.cleanup;
                        }
                        ui.checkbox(&mut self.cleanup, "")
                            .on_hover_text(cleanup_hint);

                        ui.separator();
                    });
                }

                ui.add_space(4.0);
            });
    }
}
