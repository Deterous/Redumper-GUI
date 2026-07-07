use eframe::egui;

use super::App;
use crate::theme::{ColorTheme, ThemePreset};

impl App {
    // Paint the bottom status bar panel for the theme selector and about link
    pub(super) fn paint_statusbar(&mut self, ui: &mut egui::Ui, t: &ColorTheme) {
        egui::Panel::bottom("statusbar")
            .frame(
                egui::Frame::new()
                    .fill(t.bg_panel)
                    .inner_margin(egui::Margin::same(6))
                    .stroke(egui::Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Theme selector
                    ui.label(egui::RichText::new("THEME ▸").color(t.accent).size(13.0));
                    let mut current = self.theme.unwrap_or(ThemePreset::Dark);
                    let is_light = current.is_light();
                    egui::ComboBox::from_id_salt("theme")
                        .width(100.0)
                        .selected_text(egui::RichText::new(current.name()).size(13.0).color(t.highlight))
                        .show_ui(ui, |ui| {
                            let header = if is_light { "☀ Light Themes" } else { "🌙 Dark Themes" };
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(egui::RichText::new(header).color(t.text_dim).size(11.0));
                            ui.separator();
                            for &preset in ThemePreset::ALL.iter().filter(|p| p.is_light() == is_light) {
                                ui.selectable_value(
                                    &mut current,
                                    preset,
                                    egui::RichText::new(preset.name()).size(13.0).color(t.highlight),
                                );
                            }
                        });
                    self.theme = Some(current);

                    // Button to switch between light/dark mode
                    let mode_icon = if current.is_light() { "☀" } else { "🌙" };
                    if ui.button(egui::RichText::new(mode_icon).size(13.0)).clicked() {
                        self.theme = Some(current.toggle_mode());
                    }

                    // Color picker for custom theme
                    if current.is_custom() {
                        ui.label(egui::RichText::new("COLOR ▸").color(t.accent).size(13.0));
                        egui::widgets::color_picker::color_edit_button_srgb(ui, &mut self.custom_color);
                    }

                    // About info link
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let link = ui.add(egui::Link::new(egui::RichText::new("ABOUT").color(t.text_dim).size(13.0)));
                        if link.clicked() {
                            self.show_about = !self.show_about;
                        }
                    });
                });
            });
    }

    // Paint the about info window
    pub(super) fn paint_about(&mut self, ui: &mut egui::Ui, t: &ColorTheme) {
        if !self.show_about {
            return;
        }
        let ctx = ui.ctx().clone();
        egui::Window::new(egui::RichText::new("ABOUT").color(t.accent).size(13.0))
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .default_width(420.0)
            .min_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(egui::Frame::new().fill(t.bg_panel).stroke(egui::Stroke::new(1.0, t.accent)).inner_margin(12.0))
            .show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    // Program info in title
                    ui.add(egui::Hyperlink::from_label_and_url(
                        egui::RichText::new(format!("Redumper GUI v{} by Deterous", env!("CARGO_PKG_VERSION")))
                            .color(t.highlight)
                            .size(14.0)
                            .strong(),
                        "https://github.com/Deterous/Redumper-GUI",
                    ));

                    // Close about window button
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 22.0), egui::Sense::click());
                        let color = if resp.hovered() {
                            ui.painter().rect_filled(rect, 3.0, t.accent_stop);
                            egui::Color32::WHITE
                        } else {
                            t.text_dim
                        };
                        if resp.clicked() {
                            self.show_about = false;
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
                ui.add_space(8.0);

                // Link to redump project
                ui.add(egui::Hyperlink::from_label_and_url(
                    egui::RichText::new("▸ Made for redump.info").color(t.highlight).size(13.0),
                    "https://redump.info/",
                ));

                // Link to redumper
                ui.add(egui::Hyperlink::from_label_and_url(
                    egui::RichText::new("▸ Powered by redumper by superg").color(t.highlight).size(13.0),
                    "https://github.com/superg/redumper",
                ));

                // Link to icon designer
                ui.add(egui::Hyperlink::from_label_and_url(
                    egui::RichText::new("▸ Icon by Bekoha").color(t.highlight).size(13.0),
                    "https://bsky.app/profile/bekoha.bsky.social",
                ));

                // Show copyright info for proportional font
                let font_link_titillium = ui.add(egui::Link::new(
                    egui::RichText::new(
                        "▸ Titillium Web — © 2009-2011 Accademia di Belle Arti di Urbino\n\
                             \x20   Licensed under the SIL Open Font License, Version 1.1",
                    )
                    .color(t.highlight)
                    .size(13.0),
                ));
                if font_link_titillium.clicked() {
                    self.show_ofl_titillium = !self.show_ofl_titillium;
                }
                if self.show_ofl_titillium {
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().max_height(100.0).id_salt("ofl_titillium").show(ui, |ui| {
                        let ofl_text = include_str!("../../assets/font/titillium/OFL.txt");
                        ui.add(
                            egui::TextEdit::multiline(&mut &*ofl_text)
                                .font(egui::TextStyle::Monospace)
                                .text_color(t.text_dim)
                                .desired_width(f32::INFINITY)
                                .desired_rows(5),
                        );
                    });
                }

                // Show copyright info for monospace font
                let font_link_ibmplexmono = ui.add(egui::Link::new(
                    egui::RichText::new(
                        "▸ IBM Plex Mono — © 2017 IBM Corp. with Reserved Font Name \"Plex\"\n\
                             \x20   Licensed under the SIL Open Font License, Version 1.1",
                    )
                    .color(t.highlight)
                    .size(13.0),
                ));
                if font_link_ibmplexmono.clicked() {
                    self.show_ofl_ibmplexmono = !self.show_ofl_ibmplexmono;
                }
                if self.show_ofl_ibmplexmono {
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().max_height(100.0).id_salt("ofl_ibmplexmono").show(ui, |ui| {
                        let ofl_text = include_str!("../../assets/font/ibmplexmono/OFL.txt");
                        ui.add(
                            egui::TextEdit::multiline(&mut &*ofl_text)
                                .font(egui::TextStyle::Monospace)
                                .text_color(t.text_dim)
                                .desired_width(f32::INFINITY)
                                .desired_rows(5),
                        );
                    });
                }
            });
    }

    // Paint the bottom collapsible log panel
    pub(super) fn paint_log_panel(&mut self, ui: &mut egui::Ui, t: &ColorTheme) {
        egui::Panel::bottom("log_panel")
            .frame(
                egui::Frame::new()
                    .fill(t.bg_panel)
                    .inner_margin(egui::Margin::same(8))
                    .stroke(egui::Stroke::new(1.0, t.border)),
            )
            .resizable(true)
            .default_size(100.0)
            .show(ui, |ui| {
                let mut expand_clicked = false;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("LOG").color(t.text_dim).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("CLEAR").color(t.highlight).size(12.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::new(1.0, t.highlight)),
                            )
                            .clicked()
                            && let Ok(mut log) = self.dump.log.lock()
                        {
                            log.clear();
                        }

                        let expand_label = if self.log_expanded { "COLLAPSE" } else { "EXPAND" };
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(expand_label).color(t.highlight).size(12.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::new(1.0, t.highlight)),
                            )
                            .clicked()
                        {
                            expand_clicked = true;
                        }
                    });
                });

                // Determine number of rows able to be printed in visible log panel
                let mut rows = (ui.available_height() - 19.0) / ui.text_style_height(&egui::TextStyle::Monospace);

                // Auto-toggle log panel visibility based on resize
                if rows >= 1.0 && !self.log_expanded {
                    self.log_expanded = true;
                } else if rows < 1.0 && self.log_expanded {
                    self.log_expanded = false;
                }

                // Click overrides auto-toggle
                if expand_clicked {
                    self.log_expanded = !self.log_expanded;
                    rows = 4.0;
                }

                if self.log_expanded {
                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(4.0);

                    egui::Frame::new()
                        .fill(t.log_bg)
                        .inner_margin(6.0)
                        .corner_radius(2.0)
                        .stroke(egui::Stroke::new(0.5, t.log_border))
                        .show(ui, |ui| {
                            let log = self.dump.log.lock().unwrap_or_else(|e| e.into_inner());
                            if rows < 3.0 {
                                // Single-line display, no scroll
                                let last_line = log.lines().last().unwrap_or("");
                                ui.add(
                                    egui::TextEdit::singleline(&mut &*last_line)
                                        .font(egui::TextStyle::Monospace)
                                        .text_color(t.log_text)
                                        .desired_width(f32::INFINITY),
                                );
                            } else {
                                // Multiline scrollable display
                                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut log.as_str())
                                            .font(egui::TextStyle::Monospace)
                                            .text_color(t.log_text)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows((rows as usize).max(4)),
                                    );
                                });
                            }
                        });
                }
            });
    }
}
