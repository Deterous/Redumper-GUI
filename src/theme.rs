use eframe::egui;

#[cfg(target_os = "windows")]
use winit::platform::windows::{Color, WindowExtWindows};

#[derive(Clone, Copy)]
pub struct ColorTheme {
    pub bg_dark: egui::Color32,       // Darkest background color, for titlebar and display panel
    pub bg_panel: egui::Color32,      // Slightly brighter background, for control panel and log panel
    pub accent: egui::Color32,        // Color for important labels
    pub accent_stop: egui::Color32,   // Color for stop/bad, usually red-ish
    pub accent_action: egui::Color32, // Color for start/good, usually green-ish
    pub highlight: egui::Color32,     // Color for action text/buttons
    pub text_dim: egui::Color32,
    pub text_bright: egui::Color32,
    pub border: egui::Color32,
    pub log_text: egui::Color32,
    pub log_bg: egui::Color32,
    pub log_border: egui::Color32,
    pub widget_bg: egui::Color32,
    pub widget_inactive: egui::Color32,
    pub widget_hovered: egui::Color32,
    pub widget_active: egui::Color32,
    pub monochrome: bool,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ThemePreset {
    Light,
    Dark,
    Solarized,
    Monokai,
    Catalyst,
    Cyberpunk,
    Pastel,
    Purple,
    HighContrast,
    HighContrastDark,
    Monochrome,
    MonochromeDark,
    Custom,
    CustomDark,
}

impl ThemePreset {
    pub const ALL: &[ThemePreset] = &[
        ThemePreset::Light,
        ThemePreset::Dark,
        ThemePreset::Solarized,
        ThemePreset::Monokai,
        ThemePreset::Catalyst,
        ThemePreset::Cyberpunk,
        ThemePreset::Pastel,
        ThemePreset::Purple,
        ThemePreset::HighContrast,
        ThemePreset::HighContrastDark,
        ThemePreset::Monochrome,
        ThemePreset::MonochromeDark,
        ThemePreset::Custom,
        ThemePreset::CustomDark,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Solarized => "Solarized",
            Self::Monokai => "Monokai",
            Self::Catalyst => "Catalyst",
            Self::Cyberpunk => "Cyberpunk",
            Self::Pastel => "Pastel",
            Self::Purple => "Purple Nova",
            Self::HighContrast | Self::HighContrastDark => "High Contrast",
            Self::Monochrome | Self::MonochromeDark => "Monochrome",
            Self::Custom | Self::CustomDark => "Custom",
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom | Self::CustomDark)
    }

    pub fn is_light(&self) -> bool {
        matches!(
            self,
            Self::Light
                | Self::Solarized
                | Self::Pastel
                | Self::HighContrast
                | Self::Catalyst
                | Self::Monochrome
                | Self::Custom
        )
    }

    pub fn toggle_mode(&self) -> ThemePreset {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
            Self::Solarized => Self::Monokai,
            Self::Monokai => Self::Solarized,
            Self::Catalyst => Self::Cyberpunk,
            Self::Cyberpunk => Self::Catalyst,
            Self::Pastel => Self::Purple,
            Self::Purple => Self::Pastel,
            Self::HighContrast => Self::HighContrastDark,
            Self::HighContrastDark => Self::HighContrast,
            Self::Monochrome => Self::MonochromeDark,
            Self::MonochromeDark => Self::Monochrome,
            Self::Custom => Self::CustomDark,
            Self::CustomDark => Self::Custom,
        }
    }

    pub fn colors(&self) -> ColorTheme {
        match self {
            Self::Light => ColorTheme {
                bg_dark: egui::Color32::from_rgb(210, 212, 216),
                bg_panel: egui::Color32::from_rgb(226, 228, 232),
                accent: egui::Color32::from_rgb(60, 68, 82),
                accent_stop: egui::Color32::from_rgb(160, 40, 40),
                accent_action: egui::Color32::from_rgb(70, 160, 80),
                highlight: egui::Color32::from_rgb(25, 60, 100),
                text_dim: egui::Color32::from_rgb(108, 114, 124),
                text_bright: egui::Color32::from_rgb(18, 22, 28),
                border: egui::Color32::from_rgb(172, 176, 184),
                log_text: egui::Color32::from_rgb(22, 28, 38),
                log_bg: egui::Color32::from_rgb(220, 222, 226),
                log_border: egui::Color32::from_rgb(175, 179, 187),
                widget_bg: egui::Color32::from_rgb(196, 200, 206),
                widget_inactive: egui::Color32::from_rgb(214, 217, 222),
                widget_hovered: egui::Color32::from_rgb(184, 188, 196),
                widget_active: egui::Color32::from_rgb(164, 170, 180),
                monochrome: false,
            },
            Self::Dark => ColorTheme {
                bg_dark: egui::Color32::from_rgb(22, 22, 26),
                bg_panel: egui::Color32::from_rgb(30, 30, 35),
                accent: egui::Color32::from_rgb(180, 190, 210),
                accent_stop: egui::Color32::from_rgb(200, 100, 100),
                accent_action: egui::Color32::from_rgb(60, 180, 100),
                highlight: egui::Color32::from_rgb(130, 155, 215),
                text_dim: egui::Color32::from_rgb(120, 125, 135),
                text_bright: egui::Color32::from_rgb(210, 212, 218),
                border: egui::Color32::from_rgb(50, 52, 58),
                log_text: egui::Color32::from_rgb(185, 188, 195),
                log_bg: egui::Color32::from_rgb(18, 18, 22),
                log_border: egui::Color32::from_rgb(42, 44, 50),
                widget_bg: egui::Color32::from_rgb(42, 44, 50),
                widget_inactive: egui::Color32::from_rgb(32, 33, 38),
                widget_hovered: egui::Color32::from_rgb(48, 50, 58),
                widget_active: egui::Color32::from_rgb(55, 58, 68),
                monochrome: false,
            },
            Self::Solarized => ColorTheme {
                bg_dark: egui::Color32::from_rgb(238, 232, 213),
                bg_panel: egui::Color32::from_rgb(253, 246, 227),
                accent: egui::Color32::from_rgb(42, 161, 152),
                accent_stop: egui::Color32::from_rgb(220, 50, 47),
                accent_action: egui::Color32::from_rgb(133, 153, 0),
                highlight: egui::Color32::from_rgb(203, 75, 22),
                text_dim: egui::Color32::from_rgb(101, 123, 131),
                text_bright: egui::Color32::from_rgb(7, 54, 66),
                border: egui::Color32::from_rgb(147, 161, 161),
                log_text: egui::Color32::from_rgb(7, 54, 66),
                log_bg: egui::Color32::from_rgb(238, 232, 213),
                log_border: egui::Color32::from_rgb(147, 161, 161),
                widget_bg: egui::Color32::from_rgb(238, 232, 213),
                widget_inactive: egui::Color32::from_rgb(253, 246, 227),
                widget_hovered: egui::Color32::from_rgb(238, 232, 213),
                widget_active: egui::Color32::from_rgb(147, 161, 161),
                monochrome: false,
            },
            Self::Monokai => ColorTheme {
                bg_dark: egui::Color32::from_rgb(39, 40, 34),
                bg_panel: egui::Color32::from_rgb(49, 50, 44),
                accent: egui::Color32::from_rgb(102, 217, 239),
                accent_stop: egui::Color32::from_rgb(249, 38, 114),
                accent_action: egui::Color32::from_rgb(166, 226, 46),
                highlight: egui::Color32::from_rgb(253, 151, 31),
                text_dim: egui::Color32::from_rgb(117, 113, 94),
                text_bright: egui::Color32::from_rgb(248, 248, 242),
                border: egui::Color32::from_rgb(69, 70, 64),
                log_text: egui::Color32::from_rgb(248, 248, 242),
                log_bg: egui::Color32::from_rgb(39, 40, 34),
                log_border: egui::Color32::from_rgb(69, 70, 64),
                widget_bg: egui::Color32::from_rgb(59, 60, 54),
                widget_inactive: egui::Color32::from_rgb(44, 45, 39),
                widget_hovered: egui::Color32::from_rgb(69, 70, 64),
                widget_active: egui::Color32::from_rgb(174, 129, 255),
                monochrome: false,
            },
            Self::Catalyst => ColorTheme {
                bg_dark: egui::Color32::from_rgb(255, 255, 255),
                bg_panel: egui::Color32::from_rgb(242, 244, 248),
                accent: egui::Color32::from_rgb(30, 80, 140),
                accent_stop: egui::Color32::from_rgb(220, 30, 30),
                accent_action: egui::Color32::from_rgb(0, 160, 220),
                highlight: egui::Color32::from_rgb(220, 30, 30),
                text_dim: egui::Color32::from_rgb(150, 150, 155),
                text_bright: egui::Color32::from_rgb(10, 10, 12),
                border: egui::Color32::from_rgb(190, 192, 200),
                log_text: egui::Color32::from_rgb(10, 10, 12),
                log_bg: egui::Color32::from_rgb(252, 252, 254),
                log_border: egui::Color32::from_rgb(215, 215, 220),
                widget_bg: egui::Color32::from_rgb(230, 232, 238),
                widget_inactive: egui::Color32::from_rgb(238, 240, 244),
                widget_hovered: egui::Color32::from_rgb(220, 222, 230),
                widget_active: egui::Color32::from_rgb(220, 30, 30),
                monochrome: false,
            },
            Self::Cyberpunk => ColorTheme {
                bg_dark: egui::Color32::from_rgb(10, 10, 18),
                bg_panel: egui::Color32::from_rgb(16, 16, 28),
                accent: egui::Color32::from_rgb(0, 255, 242),
                accent_stop: egui::Color32::from_rgb(255, 0, 230),
                accent_action: egui::Color32::from_rgb(180, 255, 0),
                highlight: egui::Color32::from_rgb(255, 240, 0),
                text_dim: egui::Color32::from_rgb(140, 160, 180),
                text_bright: egui::Color32::from_rgb(220, 240, 255),
                border: egui::Color32::from_rgb(0, 180, 170),
                log_text: egui::Color32::from_rgb(0, 220, 200),
                log_bg: egui::Color32::from_rgb(5, 5, 12),
                log_border: egui::Color32::from_rgb(30, 60, 60),
                widget_bg: egui::Color32::from_rgb(50, 55, 85),
                widget_inactive: egui::Color32::from_rgb(20, 22, 35),
                widget_hovered: egui::Color32::from_rgb(30, 35, 55),
                widget_active: egui::Color32::from_rgb(40, 20, 60),
                monochrome: false,
            },
            Self::Pastel => ColorTheme {
                bg_dark: egui::Color32::from_rgb(235, 230, 255),
                bg_panel: egui::Color32::from_rgb(245, 240, 255),
                accent: egui::Color32::from_rgb(130, 70, 170),
                accent_stop: egui::Color32::from_rgb(240, 100, 140),
                accent_action: egui::Color32::from_rgb(60, 200, 160),
                highlight: egui::Color32::from_rgb(200, 120, 50),
                text_dim: egui::Color32::from_rgb(130, 115, 150),
                text_bright: egui::Color32::from_rgb(40, 30, 60),
                border: egui::Color32::from_rgb(200, 185, 220),
                log_text: egui::Color32::from_rgb(45, 35, 65),
                log_bg: egui::Color32::from_rgb(240, 235, 255),
                log_border: egui::Color32::from_rgb(205, 190, 225),
                widget_bg: egui::Color32::from_rgb(220, 210, 240),
                widget_inactive: egui::Color32::from_rgb(230, 225, 248),
                widget_hovered: egui::Color32::from_rgb(210, 200, 235),
                widget_active: egui::Color32::from_rgb(190, 175, 220),
                monochrome: false,
            },
            Self::Purple => ColorTheme {
                bg_dark: egui::Color32::from_rgb(12, 8, 20),
                bg_panel: egui::Color32::from_rgb(20, 14, 35),
                accent: egui::Color32::from_rgb(180, 130, 255),
                accent_stop: egui::Color32::from_rgb(245, 200, 255),
                accent_action: egui::Color32::from_rgb(100, 60, 190),
                highlight: egui::Color32::from_rgb(230, 200, 255),
                text_dim: egui::Color32::from_rgb(130, 110, 160),
                text_bright: egui::Color32::from_rgb(220, 210, 240),
                border: egui::Color32::from_rgb(80, 50, 130),
                log_text: egui::Color32::from_rgb(190, 170, 230),
                log_bg: egui::Color32::from_rgb(8, 5, 15),
                log_border: egui::Color32::from_rgb(60, 40, 100),
                widget_bg: egui::Color32::from_rgb(45, 30, 75),
                widget_inactive: egui::Color32::from_rgb(25, 18, 45),
                widget_hovered: egui::Color32::from_rgb(55, 38, 90),
                widget_active: egui::Color32::from_rgb(70, 45, 110),
                monochrome: false,
            },
            Self::HighContrast => ColorTheme {
                bg_dark: egui::Color32::from_rgb(255, 255, 255),
                bg_panel: egui::Color32::from_rgb(245, 245, 245),
                accent: egui::Color32::from_rgb(0, 80, 180),
                accent_stop: egui::Color32::from_rgb(200, 0, 0),
                accent_action: egui::Color32::from_rgb(0, 140, 60),
                highlight: egui::Color32::from_rgb(0, 0, 0),
                text_dim: egui::Color32::from_rgb(80, 80, 80),
                text_bright: egui::Color32::from_rgb(0, 0, 0),
                border: egui::Color32::from_rgb(0, 0, 0),
                log_text: egui::Color32::from_rgb(0, 0, 0),
                log_bg: egui::Color32::from_rgb(255, 255, 255),
                log_border: egui::Color32::from_rgb(0, 0, 0),
                widget_bg: egui::Color32::from_rgb(230, 230, 230),
                widget_inactive: egui::Color32::from_rgb(240, 240, 240),
                widget_hovered: egui::Color32::from_rgb(210, 210, 210),
                widget_active: egui::Color32::from_rgb(0, 80, 180),
                monochrome: false,
            },
            Self::HighContrastDark => ColorTheme {
                bg_dark: egui::Color32::from_rgb(0, 0, 0),
                bg_panel: egui::Color32::from_rgb(15, 15, 15),
                accent: egui::Color32::from_rgb(80, 180, 255),
                accent_stop: egui::Color32::from_rgb(255, 80, 80),
                accent_action: egui::Color32::from_rgb(50, 220, 100),
                highlight: egui::Color32::from_rgb(255, 220, 50),
                text_dim: egui::Color32::from_rgb(180, 180, 180),
                text_bright: egui::Color32::from_rgb(255, 255, 255),
                border: egui::Color32::from_rgb(120, 120, 120),
                log_text: egui::Color32::from_rgb(240, 240, 240),
                log_bg: egui::Color32::from_rgb(5, 5, 5),
                log_border: egui::Color32::from_rgb(100, 100, 100),
                widget_bg: egui::Color32::from_rgb(35, 35, 35),
                widget_inactive: egui::Color32::from_rgb(20, 20, 20),
                widget_hovered: egui::Color32::from_rgb(55, 55, 55),
                widget_active: egui::Color32::from_rgb(80, 180, 255),
                monochrome: false,
            },
            Self::Monochrome => ColorTheme {
                bg_dark: egui::Color32::from_rgb(220, 220, 220),
                bg_panel: egui::Color32::from_rgb(235, 235, 235),
                accent: egui::Color32::from_rgb(60, 60, 60),
                accent_stop: egui::Color32::from_rgb(40, 40, 40),
                accent_action: egui::Color32::from_rgb(160, 160, 160),
                highlight: egui::Color32::from_rgb(0, 0, 0),
                text_dim: egui::Color32::from_rgb(130, 130, 130),
                text_bright: egui::Color32::from_rgb(10, 10, 10),
                border: egui::Color32::from_rgb(180, 180, 180),
                log_text: egui::Color32::from_rgb(20, 20, 20),
                log_bg: egui::Color32::from_rgb(225, 225, 225),
                log_border: egui::Color32::from_rgb(185, 185, 185),
                widget_bg: egui::Color32::from_rgb(200, 200, 200),
                widget_inactive: egui::Color32::from_rgb(215, 215, 215),
                widget_hovered: egui::Color32::from_rgb(190, 190, 190),
                widget_active: egui::Color32::from_rgb(170, 170, 170),
                monochrome: true,
            },
            Self::MonochromeDark => ColorTheme {
                bg_dark: egui::Color32::from_rgb(18, 18, 18),
                bg_panel: egui::Color32::from_rgb(28, 28, 28),
                accent: egui::Color32::from_rgb(180, 180, 180),
                accent_stop: egui::Color32::from_rgb(220, 220, 220),
                accent_action: egui::Color32::from_rgb(100, 100, 100),
                highlight: egui::Color32::from_rgb(240, 240, 240),
                text_dim: egui::Color32::from_rgb(110, 110, 110),
                text_bright: egui::Color32::from_rgb(230, 230, 230),
                border: egui::Color32::from_rgb(55, 55, 55),
                log_text: egui::Color32::from_rgb(200, 200, 200),
                log_bg: egui::Color32::from_rgb(14, 14, 14),
                log_border: egui::Color32::from_rgb(50, 50, 50),
                widget_bg: egui::Color32::from_rgb(45, 45, 45),
                widget_inactive: egui::Color32::from_rgb(32, 32, 32),
                widget_hovered: egui::Color32::from_rgb(55, 55, 55),
                widget_active: egui::Color32::from_rgb(70, 70, 70),
                monochrome: true,
            },
            Self::Custom | Self::CustomDark => {
                // Should not be called directly; use custom_colors_from_rgb instead
                custom_colors_from_rgb([100, 149, 237], self.is_light())
            }
        }
    }
}

// Create a custom light or dark theme based on the chosen theme color
pub fn custom_colors_from_rgb(rgb: [u8; 3], light: bool) -> ColorTheme {
    let [r, g, b] = rgb;

    // Simple sRGB luminosity of the theme color
    let lum = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;

    // Function to taken a base gray toward the theme hue at a given strength
    let tint = |base: u8, t: f32| -> egui::Color32 {
        let m = |c: u8| (base as f32 * (1.0 - t) + c as f32 * t) as u8;
        egui::Color32::from_rgb(m(r), m(g), m(b))
    };

    // Keep stop/action accent colors near red/green
    let stop = |base_r: u8, base_g: u8, base_b: u8| -> egui::Color32 {
        egui::Color32::from_rgb(
            base_r.saturating_add(r / 14).min(235),
            (base_g as u16 + g as u16 / 20).min(110) as u8,
            (base_b as u16 + b as u16 / 20).min(110) as u8,
        )
    };
    let action = |base_r: u8, base_g: u8, base_b: u8| -> egui::Color32 {
        egui::Color32::from_rgb(
            (base_r as u16 + r as u16 / 20).min(90) as u8,
            base_g.saturating_add(g / 14).min(220),
            (base_b as u16 + b as u16 / 20).min(140) as u8,
        )
    };

    if light {
        // Darken background when theme color is bright, lighten when dark
        let bg_shift = ((0.5 - lum) * 35.0) as i16; // Background shift to improve contrast
        let bg_base = (222 as i16 + bg_shift).clamp(195, 238) as u8; // Base grey for bg_dark
        let panel_base = (236 as i16 + bg_shift).clamp(210, 248) as u8; // Base grey for bg_panel

        // Smoothly darken text/labels as accent gets very bright (lum > 0.7)
        let darken = ((lum - 0.7) / 0.7).clamp(0.0, 1.0);
        let blend = |a: f32, b: f32| (a + (b - a) * darken) as u8; // Blend a toward b by darken%

        ColorTheme {
            bg_dark: tint(bg_base, 0.18),     // 18% from adaptive grey
            bg_panel: tint(panel_base, 0.14), // 14% from adaptive grey
            accent: egui::Color32::from_rgb(
                (r as f32 + (80.0 - r as f32) * darken) as u8,
                (g as f32 + (80.0 - g as f32) * darken) as u8,
                (b as f32 + (80.0 - b as f32) * darken) as u8,
            ),
            accent_stop: stop(180, 50, 50),
            accent_action: action(50, 160, 80),
            highlight: tint(blend(60.0, 20.0), 0.65),   // 65% from dark grey
            text_dim: tint(blend(115.0, 80.0), 0.14),   // 14% from mid grey
            text_bright: tint(blend(18.0, 10.0), 0.05), // 5% from near-black
            border: tint(175, 0.22),                    // 22% from light grey
            log_text: tint(blend(25.0, 15.0), 0.06),    // 6% from near-black
            log_bg: tint(panel_base.saturating_sub(8), 0.15), // 15% from adaptive grey
            log_border: tint(178, 0.18),                // 18% from light grey
            widget_bg: tint(198, 0.20),                 // 20% from light grey
            widget_inactive: tint(215, 0.15),           // 15% from light grey
            widget_hovered: tint(182, 0.28),            // 28% from mid-light grey
            widget_active: tint(140, 0.52),             // 52% from mid grey
            monochrome: false,
        }
    } else {
        // Lighten backgrounds when theme color is dark, darken when bright
        let bg_shift = ((lum - 0.5) * 18.0) as i16; // Background shift to improve contrast
        let bg_base = (18 as i16 + bg_shift).clamp(10, 30) as u8; // Base grey for bg_dark
        let panel_base = (28 as i16 + bg_shift).clamp(18, 42) as u8; // Base grey for bg_panel

        // Smoothly brighten toward readable levels as accent gets very dark (lum < 0.3)
        let brighten = ((0.3 - lum) / 0.3).clamp(0.0, 1.0);
        let blend = |a: f32, b: f32| (a + (b - a) * brighten) as u8; // Blend a toward b by brighten%

        ColorTheme {
            bg_dark: tint(bg_base, 0.10),     // 10% from adaptive dark grey
            bg_panel: tint(panel_base, 0.12), // 12% from adaptive dark grey
            accent: egui::Color32::from_rgb(
                (r as f32 + (160.0 - r as f32) * brighten) as u8,
                (g as f32 + (160.0 - g as f32) * brighten) as u8,
                (b as f32 + (160.0 - b as f32) * brighten) as u8,
            ),
            accent_stop: stop(210, 80, 80),
            accent_action: action(60, 190, 100),
            highlight: tint(blend(220.0, 240.0), 0.55), // 55% from bright grey
            text_dim: tint(blend(120.0, 180.0), 0.14),  // 14% from mid grey
            text_bright: tint(blend(218.0, 200.0), 0.06), // 6% from bright grey
            border: tint(52, 0.20),                     // 20% from dark grey
            log_text: tint(blend(195.0, 210.0), 0.08),  // 8% from bright grey
            log_bg: tint(bg_base.saturating_sub(4), 0.08), // 8% from adaptive dark grey
            log_border: tint(44, 0.18),                 // 18% from dark grey
            widget_bg: tint(44, 0.20),                  // 20% from dark grey
            widget_inactive: tint(32, 0.12),            // 12% from very dark grey
            widget_hovered: tint(54, 0.28),             // 28% from dark grey
            widget_active: tint(42, 0.55),              // 55% from dark grey
            monochrome: false,
        }
    }
}

pub fn default_theme(ctx: &egui::Context) -> ThemePreset {
    // Default to dark theme if OS is in dark mode, otherwise use light theme
    if ctx.theme() == egui::Theme::Dark { ThemePreset::Dark } else { ThemePreset::Light }
}

pub fn apply_visuals(
    ctx: &egui::Context,
    theme: &ThemePreset,
    window: &winit::window::Window,
    custom_color: Option<[u8; 3]>,
) {
    let t = if theme.is_custom() {
        custom_colors_from_rgb(custom_color.unwrap_or([100, 149, 237]), theme.is_light())
    } else {
        theme.colors()
    };

    // Use the egui light/dark theme as base
    let mut visuals = if theme.is_light() { egui::Visuals::light() } else { egui::Visuals::dark() };

    // Set most of the standard window colors
    visuals.override_text_color = Some(t.text_bright);
    visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(t.accent.r(), t.accent.g(), t.accent.b(), 60);
    visuals.selection.stroke = egui::Stroke::new(1.0, t.text_bright);
    visuals.hyperlink_color = t.highlight;
    visuals.faint_bg_color = t.widget_inactive;
    visuals.extreme_bg_color = t.bg_dark;
    visuals.code_bg_color = t.text_dim;
    visuals.warn_fg_color = t.accent_stop;
    visuals.error_fg_color = t.accent_stop;
    visuals.window_fill = t.bg_dark;
    visuals.panel_fill = t.bg_panel;

    // Set non-interactive widget colors
    visuals.widgets.noninteractive.expansion = 1.0;
    visuals.widgets.noninteractive.bg_fill = t.widget_inactive;
    visuals.widgets.noninteractive.weak_bg_fill = t.widget_inactive;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, t.border);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, t.text_dim);

    // Set inactive widget colors
    visuals.widgets.inactive.expansion = 1.0;
    visuals.widgets.inactive.bg_fill = t.widget_bg;
    visuals.widgets.inactive.weak_bg_fill = t.widget_bg;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, t.border);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, t.text_bright);

    // Set hovered widget colors
    visuals.widgets.hovered.expansion = 1.0;
    visuals.widgets.hovered.bg_fill = t.widget_hovered;
    visuals.widgets.hovered.weak_bg_fill = t.widget_hovered;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, t.accent);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, t.accent);

    // Set active widget colors
    visuals.widgets.active.expansion = 1.0;
    visuals.widgets.active.bg_fill = t.accent;
    visuals.widgets.active.weak_bg_fill = t.widget_active;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, t.accent_action);
    visuals.widgets.active.fg_stroke =
        egui::Stroke::new(1.0, if theme.is_light() { egui::Color32::WHITE } else { egui::Color32::BLACK });

    // Set open widget colors
    visuals.widgets.open.expansion = 1.0;
    visuals.widgets.open.bg_fill = t.widget_hovered;
    visuals.widgets.open.weak_bg_fill = t.widget_hovered;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, t.accent);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, t.accent);

    // Set window theme (light/dark titlebar)
    window.set_theme(Some(if theme.is_light() { winit::window::Theme::Light } else { winit::window::Theme::Dark }));

    // Set Windows-specific decorations
    #[cfg(target_os = "windows")]
    {
        window.set_title_background_color(Some(Color::from_rgb(t.bg_dark.r(), t.bg_dark.g(), t.bg_dark.b())));
        window.set_title_text_color(Color::from_rgb(t.text_bright.r(), t.text_bright.g(), t.text_bright.b()));
        window.set_border_color(Some(Color::from_rgb(t.border.r(), t.border.g(), t.border.b())));
    }

    ctx.set_visuals(visuals);
}
