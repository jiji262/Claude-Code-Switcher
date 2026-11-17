// build.rs — Track embedded UI assets and embed Windows .exe icon resource

use std::env;
use std::path::Path;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let icon_path = Path::new("resources/icons/icon.ico");
        if icon_path.exists() {
            #[cfg(windows)]
            {
                let mut res = winresource::WindowsResource::new();
                res.set_icon("resources/icons/icon.ico");
                if let Err(err) = res.compile() {
                    println!("cargo:warning=Failed to embed Windows icon resource: {}", err);
                }
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
