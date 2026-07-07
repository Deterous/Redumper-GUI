mod display;
mod fonts;
mod logic;
mod panels;
mod toolbar;

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use winit::window::Window;

use crate::drives;
use crate::dump::DumpState;
use crate::theme::{self, ColorTheme, ThemePreset};
use display::DiscState;
use fonts::FontState;

// Config to be saved between session
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Config {
    output_dir: Option<PathBuf>,
    retries: u32,
    speed: i32,
    auto_eject: bool,
    skeleton: bool,
    rings: bool,
    cleanup: bool,
    theme: Option<ThemePreset>,
    custom_color: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output_dir: None,
            retries: 1,
            speed: 16,
            auto_eject: false,
            skeleton: false,
            rings: false,
            cleanup: true,
            theme: None,
            custom_color: [100, 149, 237],
        }
    }
}

pub struct App {
    // Drive state
    pub(super) drives: Vec<(String, String)>, // List of available drives
    pub(super) selected_drive: usize,         // Currently selected drive
    prev_selected_drive: usize,               // Detect drive selection change

    // Redumper options
    pub(super) disc_name: String,           // --image-name={}
    pub(super) output_dir: Option<PathBuf>, // --image-path={}/disc_name
    pub(super) retries: u32,                // --retries={}
    pub(super) speed: i32,                  // --speed={}
    pub(super) auto_eject: bool,            // --auto-eject
    pub(super) skeleton: bool,              // --skeleton
    pub(super) rings: bool,                 // --rings
    pub(super) cleanup: bool,               // Post-process cleanup

    // Dump state
    pub(super) dump: DumpState,
    pub(super) dump_exists: bool, // Current disc name refers to an existing dump (partial or complete)
    pub(super) dump_refinable: bool, // Current disc name refers to an incomplete but refinable dump
    pub(super) complete_dump_exists: bool, // Current disc name refers to a complete dump
    pub(super) dump_just_finished: bool, // State of a dump having just been completed
    pub(super) action_pending: bool, // Button pressed, waiting for state change
    was_running: bool,            // Detect end of dump

    // UI state
    pub(super) theme: Option<ThemePreset>,     // Current theme
    pub(super) custom_color: [u8; 3],          // Custom theme base color
    prev_custom_color: [u8; 3],                // Detect custom color change
    pub(super) show_options: bool,             // Toolbar
    pub(super) show_about: bool,               // Panels
    pub(super) show_ofl_titillium: bool,       // Panels
    pub(super) show_ofl_ibmplexmono: bool,     // Panels
    pub(super) log_expanded: bool,             // Panels
    pub(super) box_colors: Vec<egui::Color32>, // Cached sector map colors
    cached_colors: Option<ColorTheme>,         // Cached theme colors
    prev_theme: Option<ThemePreset>,           // Detect theme change
    prev_disc_name: String,                    // Detect disc name change
    close_confirmed: bool,                     // Confirmed quit during dump
    last_drive_refresh: std::time::Instant,    // Cooldown for drive refresh

    // Disc display state
    pub(crate) disc: DiscState,

    // Lazy font loading
    fonts: FontState,

    // Window state for styling
    pub(super) window: Arc<Window>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Try get existing config, otherwise fallback to default
        let config: Config = cc.storage.and_then(|s| eframe::get_value(s, eframe::APP_KEY)).unwrap_or_default();

        // Default state variables
        let fonts = FontState::default();
        let dump = DumpState::new(fonts.detected.clone());

        // Keep access to window for styling decorations
        let window = cc.winit_window().expect("native window").clone();

        Self {
            // Drive state
            drives: drives::detect_drives(),
            selected_drive: 0,
            prev_selected_drive: 0,

            // Dump configuration
            disc_name: String::new(),
            output_dir: config.output_dir,
            retries: config.retries,
            speed: config.speed,
            auto_eject: config.auto_eject,
            skeleton: config.skeleton,
            rings: config.rings,
            cleanup: config.cleanup,

            // Dump state
            dump,
            dump_exists: false,
            dump_refinable: false,
            complete_dump_exists: false,
            dump_just_finished: false,
            action_pending: false,
            was_running: false,

            // UI state
            theme: config.theme,
            custom_color: config.custom_color,
            prev_custom_color: config.custom_color,
            show_options: false,
            show_about: false,
            show_ofl_titillium: false,
            show_ofl_ibmplexmono: false,
            log_expanded: true,
            box_colors: Vec::new(),
            cached_colors: None,
            prev_theme: None,
            prev_disc_name: String::new(),
            close_confirmed: false,
            last_drive_refresh: std::time::Instant::now(),

            // Disc display state
            disc: DiscState::default(),

            // Lazy font loading
            fonts,

            // Window state for styling
            window,
        }
    }
}

// Ensure redumper subprocess is terminated when the app is dropped
impl Drop for App {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.dump.child.lock()
            && let Some(mut child) = guard.take()
        {
            crate::dump::graceful_kill_blocking(&mut child);
        }
    }
}

impl eframe::App for App {
    // Save config to file
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let config = Config {
            output_dir: self.output_dir.clone(),
            retries: self.retries.clone(),
            speed: self.speed,
            auto_eject: self.auto_eject,
            skeleton: self.skeleton,
            rings: self.rings,
            cleanup: self.cleanup,
            theme: self.theme,
            custom_color: self.custom_color,
        };
        eframe::set_value(storage, eframe::APP_KEY, &config);
    }

    // Run each frame and handles state
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle OS close request (Alt+F4, native close button, etc.)
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.close_confirmed {
                let is_running = self.dump.running.lock().map(|g| *g).unwrap_or(false);
                if is_running {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    if rfd::MessageDialog::new()
                        .set_title("Dump Running!")
                        .set_description("A dump is in progress.\nAre you sure you want to quit?")
                        .set_level(rfd::MessageLevel::Warning)
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .show()
                        == rfd::MessageDialogResult::Yes
                    {
                        self.close_confirmed = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }

        // Skip logic if window is minimized
        if ctx.input(|i| i.viewport().minimized.unwrap_or(false)) {
            return;
        }

        // Resolve default theme on first frame
        let theme = *self.theme.get_or_insert_with(|| theme::default_theme(ctx));

        // Only apply visuals/style when theme or custom color changes
        let custom_changed = theme.is_custom() && self.custom_color != self.prev_custom_color;
        if self.prev_theme != Some(theme) || custom_changed {
            theme::apply_visuals(ctx, &theme, &self.window, Some(self.custom_color));
            self.box_colors.clear();
            self.prev_theme = Some(theme);
            self.prev_custom_color = self.custom_color;
            self.cached_colors = Some(if theme.is_custom() {
                theme::custom_colors_from_rgb(self.custom_color, theme.is_light())
            } else {
                theme.colors()
            });

            ctx.global_style_mut(|style| {
                style.interaction.selectable_labels = false;
            });
        }

        // Check whether a dump is in progress
        let is_running = self.dump.running.lock().map(|g| *g).unwrap_or(false);

        // Lazy-load extended fonts when non-latin text first appears
        if !self.fonts.all_loaded() {
            self.fonts.detect_in_str(&self.disc_name);
            self.fonts.maybe_load(ctx);
        }

        // Clear pending state on running transition
        if is_running != self.was_running {
            self.action_pending = false;
        }

        // Detect dump finish
        if self.was_running && !is_running {
            *self.dump.hash_progress.lock().unwrap() = None;
            self.dump_exists = self.check_dump_exists();
            self.dump_refinable = self.check_dump_refinable();
            self.complete_dump_exists = self.check_dump_complete();
            if self.dump_refinable {
                self.parse_state_file();
            } else if self.complete_dump_exists {
                self.populate_complete_dump_sectors();
            }
            self.dump_just_finished = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
        }
        self.was_running = is_running;

        // Recheck dump_exists when disc_name or drive selection changes
        if self.disc_name != self.prev_disc_name || self.selected_drive != self.prev_selected_drive {
            self.dump.clear_sectors();
            self.box_colors.clear();
            self.disc.profile_texture = None;
            self.dump_exists = self.check_dump_exists();
            self.dump_refinable = self.check_dump_refinable();
            self.complete_dump_exists = self.check_dump_complete();
            if self.dump_refinable {
                self.parse_state_file();
            } else if self.complete_dump_exists {
                self.populate_complete_dump_sectors();
            }
            self.dump_just_finished = false;
            self.prev_disc_name = self.disc_name.clone();
            self.prev_selected_drive = self.selected_drive;
        }

        // Disc constant speed
        let target_velocity = if is_running {
            // Constant speed based on drive speed
            let effective_speed = match self.speed {
                s if s < 1 => 16,
                100 => 72,
                s => s,
            };
            effective_speed as f64 / 10.0
        } else {
            // Static when idle
            0.0
        };

        // Update disc fidget spinner physics
        if !self.disc.dragging && (self.disc.velocity - target_velocity).abs() > 0.01 {
            let dt = ctx.input(|i| i.stable_dt) as f64;
            self.disc.angle += self.disc.velocity * dt;
            // Exponentially decay current velocity toward target speed
            let decay = 0.98 as f64;
            self.disc.velocity = target_velocity + (self.disc.velocity - target_velocity) * decay.powf(dt * 60.0);
            ctx.request_repaint();
        } else if !self.disc.dragging {
            self.disc.velocity = target_velocity;
            if is_running {
                let dt = ctx.input(|i| i.stable_dt) as f64;
                self.disc.angle += target_velocity * dt;
                ctx.request_repaint();
            }
        }

        // Track rotation score (skip if win already triggered)
        let delta = self.disc.angle - self.disc.prev_angle;
        self.disc.prev_angle = self.disc.angle;
        if !self.disc.win_triggered {
            self.disc.rotation_accumulator += delta;
            let full = App::ANGLE_PER_ROTATION;
            if self.disc.rotation_accumulator.abs() >= full {
                let completed = (self.disc.rotation_accumulator.abs() / full) as u128;
                self.disc.total_rotations = self.disc.total_rotations.saturating_add(completed);
                self.disc.rotation_accumulator -= self.disc.rotation_accumulator.signum() * completed as f64 * full;
            }
        }

        // Update visual angle (always in [0, TAU)) for rendering
        let delta_f32 = (delta * Self::DISC_ROTATION_SCALE) as f32;
        const WRAP: f32 = std::f32::consts::TAU * 10.0;
        self.disc.visual_angle = (self.disc.visual_angle + delta_f32) % WRAP;
        if self.disc.visual_angle < 0.0 {
            self.disc.visual_angle += WRAP;
        }

        // Keep angle small to preserve f64 precision at extreme velocities
        if self.disc.angle.abs() > 1e10 {
            self.disc.angle = 0.0;
            self.disc.prev_angle = 0.0;
        }
    }

    // Run each frame after logic for frame drawing
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Skip rendering if window is minimized
        if ui.ctx().input(|i| i.viewport().minimized.unwrap_or(false)) {
            return;
        }

        // Get the current theme
        let Some(t) = self.cached_colors else {
            return;
        };

        // Check whether a dump is in progress
        let is_running = self.dump.running.lock().map(|g| *g).unwrap_or(false);

        self.paint_toolbar(ui, &t, is_running); // Top toolbar
        self.paint_statusbar(ui, &t); // Theme picker
        self.paint_about(ui, &t); // About link / popup
        self.paint_log_panel(ui, &t); // Bottom log panel
        self.paint_display(ui, &t, is_running); // Central display panel
    }
}
