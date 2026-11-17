// build.rs — Track UI assets and platform icon resources

use std::env;
use std::path::Path;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let icon_path = Path::new("resources/icons/icon.ico");
        if icon_path.exists() {
            println!("cargo:rustc-link-arg-bins=/SUBSYSTEM:WINDOWS");
            println!("cargo:rustc-link-arg-bins=/ENTRY:mainCRTStartup");

            let rc_path = Path::new("resources/app.rc");
            if rc_path.exists() {
                println!("cargo:rerun-if-changed=resources/app.rc");
            }
        }
    }

    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/styles/tokens.css");
    println!("cargo:rerun-if-changed=ui/styles/app.css");
    println!("cargo:rerun-if-changed=ui/scripts/icons.js");
    println!("cargo:rerun-if-changed=ui/scripts/presets.js");
    println!("cargo:rerun-if-changed=ui/scripts/diff.js");
    println!("cargo:rerun-if-changed=ui/scripts/app.js");
    println!("cargo:rerun-if-changed=resources/icons/icon.svg");
    println!("cargo:rerun-if-changed=resources/icons/icon.ico");
    println!("cargo:rerun-if-changed=resources/icons/icon.icns");
}
