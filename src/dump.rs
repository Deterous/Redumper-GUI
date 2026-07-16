use std::io::Read;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;

use crate::postprocess;

// Atomic flags for lazy font loading by UI
#[derive(Clone, Default)]
pub struct ScriptDetected {
    pub japanese: Arc<AtomicBool>,
    pub chinese: Arc<AtomicBool>,
    pub korean: Arc<AtomicBool>,
}

pub fn has_japanese(c: char) -> bool {
    matches!(c,
        '\u{3040}'..='\u{309F}' | // Hiragana
        '\u{30A0}'..='\u{30FF}' | // Katakana
        '\u{31F0}'..='\u{31FF}' | // Katakana extensions
        '\u{FF65}'..='\u{FF9F}'   // Halfwidth katakana
    )
}

pub fn has_chinese(c: char) -> bool {
    matches!(c,
        '\u{2E80}'..='\u{2EFF}' | // CJK Radicals Supplement
        '\u{3400}'..='\u{4DBF}' | // CJK Unified Ext A
        '\u{4E00}'..='\u{9FFF}' | // CJK Unified Ideographs
        '\u{F900}'..='\u{FAFF}' | // CJK Compatibility Ideographs
        '\u{20000}'..='\u{2A6DF}' // CJK Unified Ext B
    )
}

pub fn has_korean(c: char) -> bool {
    matches!(c,
        '\u{1100}'..='\u{11FF}' | // Hangul Jamo
        '\u{3130}'..='\u{318F}' | // Hangul Compatibility Jamo
        '\u{AC00}'..='\u{D7AF}'   // Hangul Syllables
    )
}

// Detect extended unicode for each log character
pub fn detect_script(c: char, detected: &ScriptDetected) {
    if has_japanese(c) {
        detected.japanese.store(true, Ordering::Relaxed);
    } else if has_chinese(c) {
        detected.chinese.store(true, Ordering::Relaxed);
    } else if has_korean(c) {
        detected.korean.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum DiscProfile {
    CD,
    DVD,
    BD,
    HDDVD,
    XBOX,
    XBOX360,
    GC,
    WII,
}

// Determine if a profile should be set based on a log line
fn parse_profile(line: &str) -> Option<DiscProfile> {
    let value = line.split_once("profile:")?.1.trim();
    if value.starts_with("CD") {
        Some(DiscProfile::CD)
    } else if value.starts_with("DVD") {
        Some(DiscProfile::DVD)
    } else if value.starts_with("BD") {
        Some(DiscProfile::BD)
    } else if value.starts_with("HD DVD") {
        Some(DiscProfile::HDDVD)
    } else {
        None
    }
}

// Refine a detected profile based on additional log lines
pub fn refine_profile(line: &str, current: DiscProfile) -> Option<DiscProfile> {
    let trimmed = line.trim();
    if trimmed.starts_with("book type: NINTENDO") {
        Some(DiscProfile::WII)
    } else if trimmed.starts_with("disc size: 80mm") && current == DiscProfile::WII {
        Some(DiscProfile::GC)
    } else if trimmed.starts_with("omnidrive: XGD detected") || trimmed.starts_with("kreon: XGD detected") {
        if trimmed.contains("version: 2") || trimmed.contains("version: 3") {
            Some(DiscProfile::XBOX360)
        } else {
            Some(DiscProfile::XBOX)
        }
    } else {
        None
    }
}

// Redump system detected from INFO phase
#[derive(Clone, Copy, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum System {
    PSX,
    PS2,
    PS3,
    PS4,
    PS5,
    MCD,
    DC,
    SS,
    XBOX,
    XBOX360,
    GC,
    WII,
    PC,
    DVDVIDEO,
    AUDIOCD,
    BDVIDEO,
    HDDVDVIDEO,
}

impl System {
    // Detect from a log line like "PS2 [filename.iso]:"
    pub fn from_line(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if !trimmed.ends_with("]:") {
            return None;
        }
        let keyword = trimmed.split_once(" [")?.0;
        match keyword {
            "PSX" => Some(Self::PSX),
            "PS2" => Some(Self::PS2),
            "PS3" => Some(Self::PS3),
            "PS4" => Some(Self::PS4),
            "PS5" => Some(Self::PS5),
            "MCD" => Some(Self::MCD),
            "DC" => Some(Self::DC),
            "SS" => Some(Self::SS),
            "XBOX" => Some(Self::XBOX),
            "GC" => Some(Self::GC),
            "WII" => Some(Self::WII),
            "SecuROM" => Some(Self::PC),
            "DVD-VIDEO" => Some(Self::DVDVIDEO),
            "AUDIO-CD" => Some(Self::AUDIOCD),
            "BD-VIDEO" => Some(Self::BDVIDEO),
            "HDDVD-VIDEO" => Some(Self::HDDVDVIDEO),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PSX => "PSX",
            Self::PS2 => "PS2",
            Self::PS3 => "PS3",
            Self::PS4 => "PS4",
            Self::PS5 => "PS5",
            Self::MCD => "MCD",
            Self::DC => "DC",
            Self::SS => "SS",
            Self::XBOX => "XBOX",
            Self::GC => "GC",
            Self::WII => "WII",
            Self::XBOX360 => "XBOX360",
            Self::PC => "PC",
            Self::DVDVIDEO => "DVD-VIDEO",
            Self::AUDIOCD => "AUDIO-CD",
            Self::BDVIDEO => "BD-VIDEO",
            Self::HDDVDVIDEO => "HDDVD-VIDEO",
        }
    }

    // Refine XBOX to XBOX360 from "system: Xbox 360 (XGD...)" line
    pub fn refine(self, line: &str) -> Self {
        if self == Self::XBOX {
            let trimmed = line.trim();
            if trimmed.starts_with("system:") && trimmed.contains("Xbox 360") {
                return Self::XBOX360;
            }
        }
        self
    }
}

// Parse a disc profile from a log file
pub fn parse_profile_from_lines(lines: impl Iterator<Item = String>) -> Option<DiscProfile> {
    let mut profile: Option<DiscProfile> = None;

    for line in lines {
        let trimmed = line.trim();

        // Read profile line from log
        if profile.is_none() {
            if let Some(p) = parse_profile(trimmed) {
                profile = Some(p);
            }
            continue;
        }

        // Look for info for disc sub-type
        if let Some(refined) = refine_profile(trimmed, profile.unwrap()) {
            profile = Some(refined);
        }

        // Parsing is still needed within dump phase but not after
        if trimmed.starts_with("***") && !trimmed.starts_with("*** DUMP") {
            break;
        }
    }

    profile
}

// Sector read status for UI sector map generation
#[derive(Clone, Copy, PartialEq)]
pub enum SectorStatus {
    Unread,
    Ok,
    Error,
}

// Shared state between the UI and the dump thread.
#[derive(Clone)]
pub struct DumpState {
    pub log: Arc<Mutex<String>>,
    pub running: Arc<Mutex<bool>>,
    pub child: Arc<Mutex<Option<Child>>>,
    pub sector_states: Arc<Mutex<Vec<SectorStatus>>>,
    pub sectors_read: Arc<Mutex<usize>>,
    pub total_sectors: Arc<Mutex<usize>>,
    pub hash_progress: Arc<Mutex<Option<u8>>>,
    pub disc_profile: Arc<Mutex<Option<DiscProfile>>>,
    pub system: Arc<Mutex<Option<System>>>,
    pub script_detected: ScriptDetected,
}

impl DumpState {
    pub fn new(script_detected: ScriptDetected) -> Self {
        Self {
            log: Arc::new(Mutex::new(String::new())),
            running: Arc::new(Mutex::new(false)),
            child: Arc::new(Mutex::new(None)),
            sector_states: Arc::new(Mutex::new(Vec::new())),
            sectors_read: Arc::new(Mutex::new(0)),
            total_sectors: Arc::new(Mutex::new(0)),
            hash_progress: Arc::new(Mutex::new(None)),
            disc_profile: Arc::new(Mutex::new(None)),
            system: Arc::new(Mutex::new(None)),
            script_detected,
        }
    }

    pub fn clear_sectors(&self) {
        if let Ok(mut s) = self.sector_states.lock() {
            s.clear();
            s.shrink_to_fit();
        }
        if let Ok(mut s) = self.sectors_read.lock() {
            *s = 0;
        }
        if let Ok(mut s) = self.total_sectors.lock() {
            *s = 0;
        }
        *self.disc_profile.lock().unwrap() = None;
        *self.system.lock().unwrap() = None;
    }
}

// Tracks sector map state from redumper log stream
struct ProgressTracker {
    sector_states: Arc<Mutex<Vec<SectorStatus>>>,
    sectors_read: Arc<Mutex<usize>>,
    total_sectors: Arc<Mutex<usize>>,
    started: bool,
    prev_index: usize,
    prev_errors: usize,
    dump: bool,
    refine: bool,
    info: bool,
}

impl ProgressTracker {
    fn new(
        sector_states: Arc<Mutex<Vec<SectorStatus>>>,
        sectors_read: Arc<Mutex<usize>>,
        total_sectors: Arc<Mutex<usize>>,
    ) -> Self {
        Self {
            sector_states,
            sectors_read,
            total_sectors,
            started: false,
            prev_index: 0,
            prev_errors: 0,
            dump: false,
            refine: false,
            info: false,
        }
    }

    // Update state for each redumper log progress line
    fn update(&mut self, current_lba: isize, lba_end: isize, error_count: usize) {
        if !self.started {
            self.started = true;
            self.prev_errors = error_count;
        }

        // Grow the sector map if lba_end increased (overread)
        let num_sectors = (lba_end + 1).max(0) as usize;
        self.set_total_sectors(num_sectors);

        // Convert to sector index
        let index = current_lba.max(0) as usize;
        let errors_increased = error_count > self.prev_errors;
        let errors_decreased = error_count < self.prev_errors;
        self.prev_errors = error_count;

        let mut states = self.sector_states.lock().unwrap();
        if self.refine {
            // Refine phase has single-sector re-reads
            if index < states.len() {
                if errors_decreased {
                    states[index] = SectorStatus::Ok;
                } else if errors_increased {
                    states[index] = SectorStatus::Error;
                }
            }
        } else {
            // Dump phase may read in chunks, mark the range [prev_index..index]
            let start = self.prev_index.min(index).min(states.len());
            let end = self.prev_index.max(index).min(states.len());
            let status = if errors_increased { SectorStatus::Error } else { SectorStatus::Ok };
            for s in &mut states[start..end] {
                if *s == SectorStatus::Unread || status == SectorStatus::Error {
                    *s = status;
                }
            }
            drop(states);
            let mut sr = self.sectors_read.lock().unwrap();
            if end > *sr {
                *sr = end;
            }
        }
        self.prev_index = index;
    }

    // Called when refine phase starts, mark remainder of dump as read if dump phase ran
    fn finish_dump_phase(&mut self) {
        self.refine = true;
        self.prev_errors = 0;
        self.started = false;
        if !self.dump {
            return;
        }
        let ts = *self.total_sectors.lock().unwrap();
        if ts > 0 {
            let mut states = self.sector_states.lock().unwrap();
            let end = ts.min(states.len());
            for s in &mut states[..end] {
                if *s == SectorStatus::Unread {
                    *s = SectorStatus::Ok;
                }
            }
            drop(states);
            *self.sectors_read.lock().unwrap() = ts;
        }
    }

    // Called when "correction statistics:" line is seen, mark the refine phase as complete
    fn finish_refine_phase(&mut self) {
        let ts = *self.total_sectors.lock().unwrap();
        if ts > 0 {
            let mut states = self.sector_states.lock().unwrap();
            let end = ts.min(states.len());
            for s in &mut states[..end] {
                if *s == SectorStatus::Unread {
                    *s = SectorStatus::Ok;
                }
            }
            drop(states);
            *self.sectors_read.lock().unwrap() = ts;
        }
    }

    // Grows the sector map and updates total_sectors if lba_end increased (overread)
    fn set_total_sectors(&self, num_sectors: usize) {
        let mut ts = self.total_sectors.lock().unwrap();
        if num_sectors > *ts {
            *ts = num_sectors;
            drop(ts);
            let mut states = self.sector_states.lock().unwrap();
            states.resize(num_sectors, SectorStatus::Unread);
        }
    }

    // Parse a redumper progress line: "- [  0%] LBA:   1282/207655, errors: { SCSIs: 0, C2s: 0, Q: 0 }"
    fn parse_lba_line(line: &str) -> Option<(isize, isize, usize)> {
        let (lba_part, rest) = line.split_once("LBA:")?.1.split_once('/')?;
        let current_lba: isize = lba_part.trim().parse().ok()?;
        let lba_end: isize = rest.split(',').next()?.trim().parse().ok()?;

        let errors_section = line.split_once("errors:").map(|(_, s)| s).unwrap_or("");
        let error_count = errors_section
            .split([',', '{', '}'])
            .filter_map(|part| part.split_once(':'))
            .filter(|(key, _)| matches!(key.trim(), "SCSI" | "SCSIs" | "C2" | "C2s" | "EDC" | "EDCs"))
            .map(|(_, val)| val.trim().parse::<usize>().unwrap_or(0))
            .sum();

        Some((current_lba, lba_end, error_count))
    }

    // Get log contents since last full log line
    fn last_line(buf: &str) -> &str {
        match buf.rfind('\n') {
            Some(pos) => &buf[pos + 1..],
            None => buf,
        }
    }

    // Clear current log line from buffer
    fn clear_line(buf: &mut String) {
        match buf.rfind('\n') {
            Some(pos) => buf.truncate(pos + 1),
            None => buf.clear(),
        }
    }

    // Get most recent full log line
    fn last_completed_line(buf: &str) -> &str {
        let without_newline = &buf[..buf.len() - 1];
        let start = without_newline.rfind('\n').map(|p| p + 1).unwrap_or(0);
        &without_newline[start..]
    }
}

// Get progress of redumper hash/skeleton phases from log
fn parse_hash_progress(line: &str) -> Option<u8> {
    // Example: "/ [ 89%] hashing" or "- [100%] hashing"
    let bracket_start = line.find('[')? + 1;
    let bracket_end = line.find('%')?;
    line[bracket_start..bracket_end].trim().parse().ok()
}

// Reads redumper log output, appends to the UI log buffer, and updates the state for the UI's sector map
fn stream_log(mut log: impl Read, state: &DumpState, ctx: egui::Context) {
    let mut tracker =
        ProgressTracker::new(state.sector_states.clone(), state.sectors_read.clone(), state.total_sectors.clone());
    let mut buf = [0; 4096];

    loop {
        match log.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let text = String::from_utf8_lossy(&buf[..n]);
                let mut l = state.log.lock().unwrap();
                let mut needs_repaint = false;

                for (i, c) in text.char_indices() {
                    // Process carriage return
                    if c == '\r' {
                        // Normal redumper CRLF log line, pass
                        if text.as_bytes().get(i + 1) == Some(&b'\n') {
                            continue;
                        }
                        // Potential redumper progress update (LOGC_RF), parse LBA or hash progress
                        let last = ProgressTracker::last_line(&l);
                        if let Some((lba, total, errors)) = ProgressTracker::parse_lba_line(last) {
                            tracker.update(lba, total, errors);
                            needs_repaint = true;
                        } else if let Some(pct) = parse_hash_progress(last) {
                            *state.hash_progress.lock().unwrap() = Some(pct);
                            needs_repaint = true;
                        }
                        ProgressTracker::clear_line(&mut l);
                    } else {
                        // Append character to log
                        l.push(c);
                        // Lazy load font if needed
                        detect_script(c, &state.script_detected);
                        // Process completed lines
                        if c == '\n' {
                            needs_repaint = true;
                            let line = ProgressTracker::last_completed_line(&l);
                            // Detect profile
                            if state.disc_profile.lock().unwrap().is_none() {
                                if let Some(profile) = parse_profile(line) {
                                    *state.disc_profile.lock().unwrap() = Some(profile);
                                }
                            } else {
                                let mut dp = state.disc_profile.lock().unwrap();
                                if let Some(refined) = refine_profile(line, dp.unwrap()) {
                                    *dp = Some(refined);
                                }
                            }
                            // Detect phase changes
                            if line.trim_start().starts_with("***") {
                                if tracker.refine {
                                    tracker.finish_refine_phase();
                                }
                                if line.trim_start().starts_with("*** DUMP") {
                                    tracker.dump = true;
                                } else if line.trim_start().starts_with("*** REFINE") {
                                    tracker.finish_dump_phase();
                                } else if line.trim_start().starts_with("*** INFO") {
                                    tracker.info = true;
                                } else if line.trim_start().starts_with("*** HASH")
                                    || line.trim_start().starts_with("*** SKELETON")
                                {
                                    *state.hash_progress.lock().unwrap() = Some(0);
                                } else {
                                    *state.hash_progress.lock().unwrap() = None;
                                }
                            }
                            // Detect system from INFO phase
                            if tracker.info {
                                let mut system = state.system.lock().unwrap();
                                if *system == Some(System::XBOX) {
                                    *system = Some(System::XBOX.refine(line));
                                } else if system.is_none() {
                                    if let Some(new_system) = System::from_line(line) {
                                        *system = Some(new_system);
                                    }
                                }
                            }
                        }
                    }
                }

                // Cap log buffer
                if l.len() > 64000 {
                    let trim_at = l.len() - 64000;
                    let pos = l[trim_at..].find('\n').map(|p| trim_at + p + 1).unwrap_or(trim_at);
                    l.drain(..pos);
                }

                drop(l);
                if needs_repaint {
                    ctx.request_repaint();
                }
            }
        }
    }
}

// Attempt to gracefully terminate redumper subprocess
fn send_stop_signal(child: &Child) {
    #[cfg(unix)]
    {
        // Send SIGINT
        unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
    }
    #[cfg(windows)]
    {
        // Send Ctrl+Break event
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn AttachConsole(dwProcessId: u32) -> i32;
            fn FreeConsole() -> i32;
            fn GenerateConsoleCtrlEvent(dwCtrlEvent: u32, dwProcessGroupId: u32) -> i32;
        }
        unsafe {
            FreeConsole(); // Detach from current console
            AttachConsole(child.id()); // Attach to redumper subprocess
            GenerateConsoleCtrlEvent(1, child.id()); // CTRL_BREAK_EVENT
            FreeConsole(); // Detach from redumper subprocess
        };
    }
}

// Wait up to 5 seconds for process to exit gracefully, then force kill if needed
fn ensure_kill(child: &mut Child) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match child.try_wait() {
            // If process has ended, return
            Ok(Some(_)) => return,
            // If 5sec is up, force kill
            _ if std::time::Instant::now() >= deadline => {
                // Send SIGKILL
                child.kill().ok();
                child.wait().ok();
                return;
            }
            // Check again after 50ms
            _ => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    }
}

// Send graceful signal, then spawn a thread to wait and force kill if unresponsive
pub fn graceful_kill_async(mut child: Child) {
    send_stop_signal(&child);
    thread::spawn(move || {
        ensure_kill(&mut child);
    });
}

// Send graceful signal and block until the process exits or is force killed
pub fn graceful_kill_blocking(child: &mut Child) {
    send_stop_signal(child);
    ensure_kill(child);
}

// Run redumper as a subprocess, stream its output to the log, then trigger post processing
pub fn run_redumper(
    args: Vec<String>,
    output_path: Option<PathBuf>,
    work_dir: PathBuf,
    cleanup: bool,
    drive: Option<String>,
    state: DumpState,
    ctx: egui::Context,
) {
    // Ensure the working directory exists
    if std::fs::create_dir_all(&work_dir).is_err() {
        rfd::MessageDialog::new()
            .set_title("Error")
            .set_description(format!("Failed to create output directory:\n{}", work_dir.display()))
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
        return;
    }

    // Lock UI elements during dump
    *state.running.lock().unwrap() = true;

    thread::spawn(move || {
        // Prepare the redumper command
        let dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let name = dir.join(if cfg!(windows) { "redumper.exe" } else { "redumper" });

        // Ensure redumper is executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&name) {
                let mut perms = meta.permissions();
                if perms.mode() & 0o111 == 0 {
                    perms.set_mode(perms.mode() | 0o755);
                    std::fs::set_permissions(&name, perms).ok();
                }
            }
        }

        let mut cmd = Command::new(name);
        cmd.current_dir(&work_dir).args(&args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

        // Hide the console window for the redumper subprocess
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        // Spawn the redumper subprocess
        match cmd.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                *state.child.lock().unwrap() = Some(child);

                // Listen to stderr on a separate thread
                let state2 = state.clone();
                let ctx2 = ctx.clone();
                let stderr_handle = thread::spawn(move || {
                    if let Some(stderr) = stderr {
                        stream_log(stderr, &state2, ctx2);
                    }
                });

                // Listen to stdout on this thread
                if let Some(stdout) = stdout {
                    stream_log(stdout, &state, ctx.clone());
                }

                // Wait for stderr thread to finish
                stderr_handle.join().ok();

                // Get the exit status and print the exit code
                let status = state.child.lock().unwrap().take().and_then(|mut c| c.wait().ok());
                let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);

                // If dump succeeded, postprocess the output files
                if code == 0
                    && cleanup
                    && let Some(ref path) = output_path
                    && let Some(parent) = path.parent()
                {
                    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let profile = *state.disc_profile.lock().unwrap();
                    let system = *state.system.lock().unwrap();
                    postprocess::run(&ctx, &state.log, parent, &stem, drive.as_deref(), profile, system);
                } else {
                    state.log.lock().unwrap().push_str(&format!("\n[Redumper exited with code {}]\n", code));
                }
            }
            // Print error message if redumper was never spawned
            Err(e) => {
                state.log.lock().unwrap().push_str(&format!("[Failed to launch: {}]\n", e));
            }
        }

        // Dumping complete, reset UI
        *state.running.lock().unwrap() = false;
        ctx.request_repaint();
    });
}
