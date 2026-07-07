use std::sync::atomic::Ordering;

use eframe::egui;

use crate::dump::{ScriptDetected, detect_script};

// Extended font groups already loaded
pub(crate) struct FontState {
    japanese_loaded: bool,
    chinese_loaded: bool,
    korean_loaded: bool,
    pub detected: ScriptDetected,
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            japanese_loaded: false,
            chinese_loaded: false,
            korean_loaded: false,
            detected: ScriptDetected::default(),
        }
    }
}

fn japanese_font_paths() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["C:\\Windows\\Fonts\\YuGothM.ttc", "C:\\Windows\\Fonts\\msgothic.ttc", "C:\\Windows\\Fonts\\meiryo.ttc"]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/\u{30D2}\u{30E9}\u{30AE}\u{30CE}\u{89D2}\u{30B4}\u{30B7}\u{30C3}\u{30AF} W3.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        ]
    }
}

fn chinese_font_paths() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["C:\\Windows\\Fonts\\msyh.ttc", "C:\\Windows\\Fonts\\simsun.ttc", "C:\\Windows\\Fonts\\mingliub.ttc"]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans TC.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        ]
    }
}

fn korean_font_paths() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["C:\\Windows\\Fonts\\malgun.ttf", "C:\\Windows\\Fonts\\gulim.ttc", "C:\\Windows\\Fonts\\batang.ttc"]
    } else if cfg!(target_os = "macos") {
        &["/System/Library/Fonts/AppleSDGothicNeo.ttc", "/Library/Fonts/AppleGothic.ttf"]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/adobe-source-han/SourceHanSans.ttc",
            "/usr/share/fonts/opentype/source-han-sans/SourceHanSans.ttc",
        ]
    }
}

// Try to load the first available font from a list of paths
fn load_first_available(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| std::fs::read(p).ok())
}

impl FontState {
    // Check atomic flags and load fonts for any newly-detected scripts
    pub fn maybe_load(&mut self, ctx: &egui::Context) {
        let need_japanese = !self.japanese_loaded && self.detected.japanese.load(Ordering::Relaxed);
        let need_chinese = !self.chinese_loaded && self.detected.chinese.load(Ordering::Relaxed);
        let need_korean = !self.korean_loaded && self.detected.korean.load(Ordering::Relaxed);

        if !need_japanese && !need_chinese && !need_korean {
            return;
        }

        let mut extra_fonts: Vec<(String, Vec<u8>)> = Vec::new();
        let mut idx = 0;

        // Collect already-loaded fonts
        if self.japanese_loaded {
            if let Some(bytes) = load_first_available(japanese_font_paths()) {
                extra_fonts.push((format!("Font{idx}"), bytes));
                idx += 1;
            }
        }
        if self.chinese_loaded {
            if let Some(bytes) = load_first_available(chinese_font_paths()) {
                extra_fonts.push((format!("Font{idx}"), bytes));
                idx += 1;
            }
        }
        if self.korean_loaded {
            if let Some(bytes) = load_first_available(korean_font_paths()) {
                extra_fonts.push((format!("Font{idx}"), bytes));
                idx += 1;
            }
        }

        // Load newly needed fonts
        if need_japanese {
            if let Some(bytes) = load_first_available(japanese_font_paths()) {
                extra_fonts.push((format!("Font{idx}"), bytes));
                idx += 1;
                self.japanese_loaded = true;
            }
        }
        if need_chinese {
            if let Some(bytes) = load_first_available(chinese_font_paths()) {
                extra_fonts.push((format!("Font{idx}"), bytes));
                idx += 1;
                self.chinese_loaded = true;
            }
        }
        if need_korean {
            if let Some(bytes) = load_first_available(korean_font_paths()) {
                extra_fonts.push((format!("Font{idx}"), bytes));
                let _ = idx;
                self.korean_loaded = true;
            }
        }

        if !extra_fonts.is_empty() {
            // Create a new set of fonts by appending the extra fonts to the default fonts
            let mut fonts = egui::FontDefinitions::default();

            // Use Titillium Web proportional font
            fonts.font_data.insert(
                "Titillium".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../../assets/font/titillium/TitilliumWeb-Regular.ttf"
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
                    "../../assets/font/ibmplexmono/IBMPlexMono-Regular.ttf"
                ))),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "IBMPlexMono".to_owned());
                family.insert(1, "Hack".to_owned());
            }

            // Append the extra fonts
            for (name, bytes) in extra_fonts {
                fonts.font_data.insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes.clone())));
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    family.push(name.clone());
                }
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    family.push(name.clone());
                }
            }

            ctx.set_fonts(fonts);
        }
    }

    // Function to check for extended characters in disc name input field
    pub fn detect_in_str(&self, text: &str) {
        for c in text.chars() {
            detect_script(c, &self.detected);
        }
    }

    // Shortcut if all fonts have already been loaded
    pub fn all_loaded(&self) -> bool {
        self.japanese_loaded && self.chinese_loaded && self.korean_loaded
    }
}
