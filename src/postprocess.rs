use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use eframe::egui;

const ZSTD_EXTENSIONS: &[&str] = &["state", "skeleton", "subcode"];
const KEEP_EXTENSIONS: &[&str] = &["bin", "cue", "iso"];

// Redumper post-process files
pub fn run(dir: &Path, image_name: &str, log: &Arc<Mutex<String>>, ctx: &egui::Context) {
    // Delete temp files
    let mut delete = vec!["cache"];
    if dir.join(format!("{}.cue", image_name)).exists() {
        delete.push("scram");
    }
    if dir.join(format!("{}.iso", image_name)).exists() {
        delete.push("sdram");
        delete.push("sbram");
    }

    for ext in delete {
        let file = dir.join(format!("{}.{}", image_name, ext));
        if file.exists() {
            match fs::remove_file(&file) {
                Ok(_) => log
                    .lock()
                    .unwrap()
                    .push_str(&format!("  Deleted: {}\n", file.file_name().unwrap().to_string_lossy())),
                Err(e) => log.lock().unwrap().push_str(&format!(
                    "  Failed to delete {}: {}\n",
                    file.file_name().unwrap().to_string_lossy(),
                    e
                )),
            }
        }
    }

    // Zstd-compress state/skeleton/subcode
    for ext in ZSTD_EXTENSIONS {
        let file = dir.join(format!("{}.{}", image_name, ext));
        if file.exists() {
            let out_path = dir.join(format!("{}.{}.zst", image_name, ext));
            match zstd_compress(&file, &out_path) {
                Ok(_) => {
                    fs::remove_file(&file).ok();
                    log.lock().unwrap().push_str(&format!("  Compressed: {}.{} -> .{}.zst\n", image_name, ext, ext));
                }
                Err(e) => {
                    log.lock().unwrap().push_str(&format!("  Failed to compress {}.{}: {}\n", image_name, ext, e));
                }
            }
        }
    }

    // Zip remaining files (except bin/cue/iso)
    let zip_path = dir.join(format!("{}_logs.zip", image_name));
    match zip_logs(dir, image_name, &zip_path) {
        Ok(count) if count > 0 => {
            log.lock().unwrap().push_str(&format!("  Archived {} file(s) into {}_logs.zip\n", count, image_name));
        }
        Ok(_) => {}
        Err(e) => {
            log.lock().unwrap().push_str(&format!("  Failed to create zip: {}\n", e));
        }
    }

    ctx.request_repaint();
}

// Compress a file with zstd
fn zstd_compress(input: &Path, output: &Path) -> std::io::Result<()> {
    // Write input file to zstd compressed output path
    let reader = fs::File::open(input)?;
    let writer = fs::File::create(output)?;
    let mut encoder = zstd::Encoder::new(writer, 3)?;
    std::io::copy(&mut std::io::BufReader::new(reader), &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

// Get a zip-format datetime of a file
fn file_datetime(path: &Path) -> zip::DateTime {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| {
            let odt: time::OffsetDateTime = t.into();
            zip::DateTime::try_from(time::PrimitiveDateTime::new(odt.date(), odt.time())).ok()
        })
        .unwrap_or_default()
}

// Compress a folder of redumper output files to a zip
fn zip_logs(dir: &Path, image_name: &str, zip_path: &Path) -> std::io::Result<usize> {
    // Filter all files in current directory
    let entries: Vec<_> = fs::read_dir(dir)?
        .flatten()
        .filter(|e| {
            let path = e.path();
            // Skip folders
            if !path.is_file() {
                return false;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            // Only include files that belong to this dump
            if !name.starts_with(image_name) {
                return false;
            }
            // Exclude bin/cue/iso
            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if KEEP_EXTENSIONS.contains(&ext.as_str()) {
                return false;
            }
            // Don't zip itself
            if path == zip_path {
                return false;
            }
            true
        })
        .collect();

    // Return 0 if no zippable files were found
    if entries.is_empty() {
        return Ok(0);
    }

    // Create zip file with deflate
    let file = fs::File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    // Add each file to zip
    let mut count = 0;
    for entry in &entries {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(file_datetime(&path));
        let mut f = std::io::BufReader::new(fs::File::open(&path)?);
        zip.start_file(&name, options)?;
        std::io::copy(&mut f, &mut zip)?;
        count += 1;
    }

    // Close zip file
    zip.finish()?;

    // Delete files that were zipped from folder (except .log)
    for entry in &entries {
        let path = entry.path();
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext != "log" {
            fs::remove_file(path).ok();
        }
    }

    // Return the number of zipped files
    Ok(count)
}
