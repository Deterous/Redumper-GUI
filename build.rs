use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    if target_os == "windows" {
        let out_dir = env::var("OUT_DIR").unwrap();
        let parts: Vec<&str> = version.split('.').collect();
        let major = parts.get(0).unwrap_or(&"0");
        let minor = parts.get(1).unwrap_or(&"0");
        let patch = parts.get(2).unwrap_or(&"0");

        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let ico_path = Path::new(&manifest_dir).join("assets/icon/icon.ico");
        let ico_path_escaped = ico_path.to_str().unwrap().replace('\\', "/");

        let rc_content = format!(
            r#"1 ICON "{ico_path_escaped}"

#include <winver.h>

VS_VERSION_INFO VERSIONINFO
FILEVERSION    {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEFLAGSMASK  VS_FFI_FILEFLAGSMASK
FILEFLAGS      0
FILEOS         VOS__WINDOWS32
FILETYPE       VFT_APP
FILESUBTYPE    VFT2_UNKNOWN
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "FileDescription", "Redumper GUI"
            VALUE "FileVersion", "{version}"
            VALUE "InternalName", "redumper-gui"
            VALUE "OriginalFilename", "redumper-gui.exe"
            VALUE "ProductName", "Redumper GUI"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 0x04B0
    END
END
"#
        );

        let rc_path = Path::new(&out_dir).join("icon.rc");
        fs::write(&rc_path, rc_content).unwrap();
        let _ = embed_resource::compile(rc_path, embed_resource::NONE);
    }

    if target_os == "macos" {
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Redumper GUI</string>
    <key>CFBundleDisplayName</key>
    <string>Redumper GUI</string>
    <key>CFBundleIdentifier</key>
    <string>com.redumper.gui</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleExecutable</key>
    <string>redumper-gui</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleIconFile</key>
    <string>icon</string>
</dict>
</plist>
"#
        );

        let plist_path = Path::new("Info.plist");
        fs::write(plist_path, plist_content).unwrap();
    }
}
