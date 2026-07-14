#[cfg(not(target_os = "macos"))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(not(target_os = "macos"))]
use std::sync::OnceLock;

use super::App;
use crate::dump::{self, SectorStatus};

impl App {
    // .state file LBA 0 offsets
    const CD_LBA_OFFSET: u64 = 26548200;
    const DVD_LBA_OFFSET: u64 = 0x30000;
    const BD_LBA_OFFSET: u64 = 0x100000;

    // Whether the user running us may create a file in this directory. Probes by creating one,
    // so that ownership, ACLs and read-only mounts all count; the mode bits alone answer a
    // different question. Only the creation decides: if the cleanup fails the directory is
    // still writable.
    #[cfg(not(target_os = "macos"))]
    fn dir_is_writable(dir: &Path) -> bool {
        let probe = dir.join(".redumper-gui-write-test");
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                true
            }
            Err(_) => false,
        }
    }

    fn default_output_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            // macOS executable is within .app file, default to Documents/Dumps instead
            dirs::download_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))).join("Dumps")
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Default to a Dumps subfolder next to the executable, which is what a portable build
            // unpacked by the user gives us. Installed from a package the executable lives in a
            // system directory instead, where creating that folder fails with EACCES, so fall back
            // to the user's home. Probed once and remembered: effective_output_dir() runs on every
            // frame.
            static DEFAULT: OnceLock<PathBuf> = OnceLock::new();
            DEFAULT
                .get_or_init(|| match std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
                    Some(dir) if Self::dir_is_writable(&dir) => dir.join("Dumps"),
                    _ => dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Dumps"),
                })
                .clone()
        }
    }

    // The currently defined output directory
    pub(super) fn effective_output_dir(&self) -> PathBuf {
        let base = match &self.output_dir {
            Some(dir) => dir.clone(),
            None => Self::default_output_dir(),
        };
        if self.disc_name.is_empty() { base } else { base.join(&self.disc_name) }
    }

    // The full output path including base filename
    pub(super) fn output_path(&self) -> PathBuf {
        let dir = self.effective_output_dir();
        if self.disc_name.is_empty() { dir } else { dir.join(&self.disc_name) }
    }

    // Build full redumper command to run
    pub(super) fn build_args(&self, refine: bool) -> Vec<String> {
        let mut args = vec![if refine { "--continue=refine" } else { "disc" }.to_string()];
        args.push("--auto-detect".to_string());
        if refine {
            args.push("--overwrite".to_string());
        }
        if let Some((drive, _)) = self.drives.get(self.selected_drive) {
            args.push(format!("--drive={}", drive));
        }

        if !self.disc_name.is_empty() {
            args.push(format!("--image-name={}", self.disc_name));
        }
        args.push(format!("--retries={}", self.retries));
        if self.speed >= 0 && self.speed < 100 {
            args.push(format!("--speed={}", self.speed));
        }
        if self.auto_eject {
            args.push("--auto-eject".to_string());
        }
        if self.skeleton {
            args.push("--skeleton".to_string());
        }
        if self.rings {
            args.push("--rings".to_string());
        }
        args
    }

    // Determine whether at least a partial dump exists in the current directory
    pub(super) fn check_dump_exists(&self) -> bool {
        if self.disc_name.is_empty() {
            return false;
        }
        let dir = self.effective_output_dir();
        ["cue", "iso", "state", "log"].iter().any(|ext| dir.join(format!("{}.{}", self.disc_name, ext)).exists())
            || dir.join(format!("{}_logs.zip", self.disc_name)).exists()
    }

    // Determine whether a complete dump exists in the current directory (no state file)
    pub(super) fn check_dump_complete(&self) -> bool {
        if self.disc_name.is_empty() {
            return false;
        }
        let dir = self.effective_output_dir();
        !dir.join(format!("{}.state", self.disc_name)).exists()
            && (["cue", "iso"].iter().any(|ext| dir.join(format!("{}.{}", self.disc_name, ext)).exists())
                || dir.join(format!("{}_logs.zip", self.disc_name)).exists())
    }

    // Determine whether the current path has a dump that can be refined
    pub(super) fn check_dump_refinable(&self) -> bool {
        if self.disc_name.is_empty() {
            return false;
        }
        let dir = self.effective_output_dir();
        let state_path = dir.join(format!("{}.state", self.disc_name));
        if !state_path.exists() {
            return false;
        }
        if !dir.join(format!("{}.log", self.disc_name)).exists() {
            return false;
        }

        let has_scram = dir.join(format!("{}.scram", self.disc_name)).exists();
        let has_sdram = dir.join(format!("{}.sdram", self.disc_name)).exists();
        let has_sbram = dir.join(format!("{}.sbram", self.disc_name)).exists();
        let has_iso = dir.join(format!("{}.iso", self.disc_name)).exists();

        // No more than one scrambled dump can exist
        if (has_scram as u8 + has_sdram as u8 + has_sbram as u8) > 1 {
            return false;
        }

        // scram + iso cannot coexist
        if has_scram && has_iso {
            return false;
        }

        // Need at least one data file
        if !has_scram && !has_sdram && !has_sbram && !has_iso {
            return false;
        }

        let offset = if has_scram {
            Self::CD_LBA_OFFSET
        } else if has_sdram {
            Self::DVD_LBA_OFFSET
        } else if has_sbram {
            Self::BD_LBA_OFFSET
        } else {
            0
        };
        state_path.metadata().map(|m| m.len() > offset).unwrap_or(false)
    }

    // Get the dump's state file offset, depending on existence of scram files
    fn state_file_offset(&self) -> u64 {
        let dir = self.effective_output_dir();
        if dir.join(format!("{}.scram", self.disc_name)).exists() {
            Self::CD_LBA_OFFSET
        } else if dir.join(format!("{}.sdram", self.disc_name)).exists() {
            Self::DVD_LBA_OFFSET
        } else if dir.join(format!("{}.sbram", self.disc_name)).exists() {
            Self::BD_LBA_OFFSET
        } else {
            0
        }
    }

    // Initialize the display state by parsing the log
    pub(super) fn populate_complete_dump_sectors(&mut self) {
        if let Some(count) = self.parse_sector_count_from_log() {
            let states = vec![SectorStatus::Ok; count];
            *self.dump.sector_states.lock().unwrap() = states;
            *self.dump.sectors_read.lock().unwrap() = count;
            *self.dump.total_sectors.lock().unwrap() = count;
        }
        self.detect_profile_from_log();
    }

    // Determine the total sector count of a disc by parsing an existing log file
    fn parse_sector_count_from_log(&self) -> Option<usize> {
        use std::io::{BufRead, BufReader};
        let dir = self.effective_output_dir();
        let log_path = dir.join(format!("{}.log", self.disc_name));
        let file = std::fs::File::open(log_path).ok()?;
        let reader = BufReader::new(file);

        let mut in_dump = false;
        let mut sector_count: Option<usize> = None;
        let mut looking_for_toc_lba = false;
        let mut looking_for_next_session = false;
        let mut have_capacity = false;

        for line in reader.lines() {
            let line = line.ok()?;
            let trimmed = line.trim();

            if !in_dump {
                if trimmed.starts_with("*** DUMP") {
                    in_dump = true;
                }
                continue;
            }

            // Stop reading if we hit another *** or === section
            if trimmed.starts_with("***") || trimmed.starts_with("===") {
                break;
            }

            // DVD/BD Xbox override: "sectors count (XBOX): N"
            if let Some(rest) = trimmed.strip_prefix("sectors count (XBOX): ") {
                return rest.split_whitespace().next().and_then(|s| s.parse().ok());
            }

            // If we already have READ_CAPACITY and this line isn't XBOX, return it
            if have_capacity {
                return sector_count;
            }

            // DVD/BD: "sectors count (READ_CAPACITY): N"
            if let Some(rest) = trimmed.strip_prefix("sectors count (READ_CAPACITY): ") {
                sector_count = rest.split_whitespace().next().and_then(|s| s.parse().ok());
                have_capacity = true;
                continue;
            }

            // CD TOC: look for "track A" or "track AA"
            if trimmed.starts_with("track A ") || trimmed.starts_with("track AA ") {
                looking_for_toc_lba = true;
                looking_for_next_session = false;
                continue;
            }

            if looking_for_toc_lba && trimmed.starts_with("index 01") {
                // Parse LBA from: index 01 { LBA: 185749, MSF: ... }
                if let Some(lba_start) = trimmed.find("LBA:") {
                    let after = &trimmed[lba_start + 4..];
                    let lba_str = after.trim_start().split(|c: char| !c.is_ascii_digit()).next().unwrap_or("");
                    if let Ok(lba) = lba_str.parse::<usize>() {
                        sector_count = Some(lba);
                    }
                }
                looking_for_toc_lba = false;
                looking_for_next_session = true;
                continue;
            }

            if looking_for_next_session {
                if trimmed.starts_with("session ") {
                    // More sessions ahead, keep looking for next track A/AA
                    looking_for_next_session = false;
                    continue;
                } else if trimmed.starts_with("track A ") || trimmed.starts_with("track AA ") {
                    looking_for_toc_lba = true;
                    looking_for_next_session = false;
                    continue;
                } else if !trimmed.is_empty() {
                    looking_for_next_session = false;
                }
            }
        }

        sector_count
    }

    // Initialize sector map state by reading the state file
    pub(super) fn parse_state_file(&mut self) {
        let offset = self.state_file_offset();
        let dir = self.effective_output_dir();
        let state_path = dir.join(format!("{}.state", self.disc_name));
        let data = match std::fs::read(&state_path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let sector_data = &data[offset as usize..];
        let samples_per_sector = if offset == Self::CD_LBA_OFFSET { 588 } else { 1 };
        let state_sectors = sector_data.len() / samples_per_sector;
        if state_sectors == 0 {
            return;
        }

        let total = self.parse_sector_count_from_log().unwrap_or(state_sectors).max(state_sectors);

        let mut states = Vec::with_capacity(total);
        if samples_per_sector == 1 {
            for &b in sector_data {
                states.push(match b {
                    0 => SectorStatus::Unread,
                    1 => SectorStatus::Error,
                    _ => SectorStatus::Ok,
                });
            }
        } else {
            for chunk in sector_data.chunks(samples_per_sector) {
                states.push(if chunk.contains(&1) {
                    SectorStatus::Error
                } else if chunk.contains(&0) {
                    SectorStatus::Unread
                } else {
                    SectorStatus::Ok
                });
            }
        }
        states.resize(total, SectorStatus::Unread);

        *self.dump.sector_states.lock().unwrap() = states;
        *self.dump.sectors_read.lock().unwrap() = state_sectors;
        *self.dump.total_sectors.lock().unwrap() = total;

        self.detect_profile_from_log();
    }

    // Detect the disc profile by reading the log file
    fn detect_profile_from_log(&mut self) {
        use std::io::{BufRead, BufReader};
        let dir = self.effective_output_dir();
        let log_path = dir.join(format!("{}.log", self.disc_name));
        let file = match std::fs::File::open(log_path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let lines = BufReader::new(file).lines().map_while(Result::ok);
        if let Some(profile) = dump::parse_profile_from_lines(lines) {
            *self.dump.disc_profile.lock().unwrap() = Some(profile);
        }
    }
}
