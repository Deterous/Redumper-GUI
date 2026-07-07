// Enumerate available optical drives
pub fn detect_drives() -> Vec<(String, String)> {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetLogicalDrives() -> u32;
            fn GetDriveTypeA(lpRootPathName: *const u8) -> u32;
            fn GetVolumeInformationA(
                lpRootPathName: *const u8,
                lpVolumeNameBuffer: *mut u8,
                nVolumeNameSize: u32,
                lpVolumeSerialNumber: *mut u32,
                lpMaximumComponentLength: *mut u32,
                lpFileSystemFlags: *mut u32,
                lpFileSystemNameBuffer: *mut u8,
                nFileSystemNameSize: u32,
            ) -> i32;
        }

        // Get bitmask of all drive letters present on the system
        let mask = unsafe { GetLogicalDrives() };

        // Check each existing drive letter for optical drive type
        (0..26)
            .filter(|i| mask & (1 << i) != 0)
            .filter_map(|i| {
                let letter = (b'A' + i as u8) as char;
                let root = format!("{}:\\\0", letter);
                if unsafe { GetDriveTypeA(root.as_ptr()) } == 5 {
                    // Try get volume label from drive
                    let mut volume_name_buf = [0; 261];
                    let success = unsafe {
                        GetVolumeInformationA(
                            root.as_ptr(),
                            volume_name_buf.as_mut_ptr(),
                            volume_name_buf.len() as u32,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            0,
                        )
                    };

                    let volume_label = if success != 0 {
                        let len = volume_name_buf.iter().position(|&x| x == 0).unwrap_or(0);
                        String::from_utf8_lossy(&volume_name_buf[..len]).into_owned()
                    } else {
                        String::new()
                    };

                    Some((format!("{}:", letter), volume_label))
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let mut drives = Vec::new();
        let mut fallback_drives = Vec::new();

        // Use diskutil list to get list of all drives on system
        let Ok(list_output) = Command::new("diskutil").args(["list"]).output() else {
            return Vec::new();
        };

        // Get list of candidate drives (ignore virtual/synthesized disks)
        let nodes: Vec<String> = String::from_utf8_lossy(&list_output.stdout)
            .lines()
            .filter_map(|line| {
                let first = line.split_whitespace().next()?;
                let node = first.strip_prefix("/dev/")?.trim_end_matches(':').to_string();
                let lower = line.to_ascii_lowercase();
                if lower.contains("virtual") || lower.contains("synthesized") { None } else { Some(node) }
            })
            .collect();

        // Use diskutil info to get detailed information for each candidate drive
        for node in nodes {
            let Ok(output) = Command::new("diskutil").args(["info", &format!("/dev/{}", node)]).output() else {
                continue;
            };
            let info = String::from_utf8_lossy(&output.stdout);

            let mut optical = false;
            let mut ejectable = false;
            let mut removable = false;
            let mut volume_name = String::new();

            for line in info.lines() {
                let t = line.trim();
                if t.starts_with("Optical Drive Type:") || t.starts_with("Optical Media Type:") {
                    optical = true;
                } else if let Some(val) = t.strip_prefix("Ejectable:") {
                    ejectable = val.trim().eq_ignore_ascii_case("Yes");
                } else if let Some(val) = t.strip_prefix("Removable Media:") {
                    removable = val.trim().eq_ignore_ascii_case("Removable");
                } else if let Some(val) = t.strip_prefix("Volume Name:") {
                    let val = val.trim();
                    if !val.is_empty() && val != "Not applicable" {
                        volume_name = val.to_string();
                    }
                }
            }

            // Optical drives are guaranteed, but keep track of ejectable/removable as fallback
            if optical {
                drives.push((node, volume_name));
            } else if ejectable && removable {
                fallback_drives.push((node, volume_name));
            }
        }

        // If no optical drives found, fall back to ejectable/removable drives just in case
        if drives.is_empty() {
            drives = fallback_drives;
        }

        // Sort drives and return list
        drives.sort_by(|a, b| a.0.cmp(&b.0));
        drives
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::process::Command;

        let mut drives = Vec::new();

        // Scan sysfs for SCSI type 5 (CD-ROM) devices
        let sysfs_dirs = [
            "/sys/subsystem/scsi/devices",
            "/sys/bus/scsi/devices",
            "/sys/class/scsi/devices",
            "/sys/block/scsi/devices",
        ];

        for sysfs_dir in &sysfs_dirs {
            let dir = std::path::Path::new(sysfs_dir);
            if !dir.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }

                    // Check SCSI device type 5 (CDROM)
                    let type_path = path.join("type");
                    if let Ok(contents) = fs::read_to_string(&type_path) {
                        if contents.trim() != "5" {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    // Find the generic SCSI device name (preferred)
                    let sg_path = path.join("scsi_generic");
                    if sg_path.is_dir() {
                        if let Ok(sg_entries) = fs::read_dir(&sg_path) {
                            if let Some(sg_entry) = sg_entries.flatten().next() {
                                if sg_entry.path().is_dir() {
                                    let dev_path = format!("/dev/{}", sg_entry.file_name().to_string_lossy());
                                    drives.push((dev_path, String::new()));
                                    continue;
                                }
                            }
                        }
                    }

                    // Fall back to block device (sr) if no sg device found
                    let block_path = path.join("block");
                    if block_path.is_dir() {
                        if let Ok(block_entries) = fs::read_dir(&block_path) {
                            if let Some(block_entry) = block_entries.flatten().next() {
                                if block_entry.path().is_dir() {
                                    let dev_path = format!("/dev/{}", block_entry.file_name().to_string_lossy());

                                    // Try get volume label from lsblk
                                    let mut volume_label = String::new();
                                    if let Ok(output) =
                                        Command::new("lsblk").args(["-d", "-n", "-o", "LABEL", &dev_path]).output()
                                    {
                                        let text = String::from_utf8_lossy(&output.stdout);
                                        let trimmed = text.trim();
                                        if !trimmed.is_empty() {
                                            volume_label = trimmed.to_string();
                                        }
                                    }
                                    drives.push((dev_path, volume_label));
                                }
                            }
                        }
                    }
                }
            }

            // Use the first sysfs directory that exists
            break;
        }

        // Fallback by scanning /dev for sg and sr entries
        if drives.is_empty() {
            if let Ok(entries) = fs::read_dir("/dev") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_sg = name.starts_with("sg") && name[2..].chars().all(|c| c.is_ascii_digit());
                    let is_sr = name.starts_with("sr") && name[2..].chars().all(|c| c.is_ascii_digit());
                    if is_sg || is_sr {
                        let dev_path = format!("/dev/{}", name);

                        // Try get volume label from lsblk
                        let mut volume_label = String::new();
                        if is_sr {
                            if let Ok(output) =
                                Command::new("lsblk").args(["-d", "-n", "-o", "LABEL", &dev_path]).output()
                            {
                                let text = String::from_utf8_lossy(&output.stdout);
                                let trimmed = text.trim();
                                if !trimmed.is_empty() {
                                    volume_label = trimmed.to_string();
                                }
                            }
                        }
                        drives.push((dev_path, volume_label));
                    }
                }
            }
        }

        // Sort drives and return list
        drives.sort_by(|a, b| a.0.cmp(&b.0));
        drives
    }
}
