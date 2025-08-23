// build.rs - 构建时生成图标文件

use std::env;
use std::path::Path;

fn main() {
    // 只在 Windows 平台上设置图标
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // 检查是否存在 icon.ico 文件
        let icon_path = Path::new("resources/icons/icon.ico");
        if icon_path.exists() {
            // 设置 Windows 图标
            println!("cargo:rustc-link-arg-bins=/SUBSYSTEM:WINDOWS");
            println!("cargo:rustc-link-arg-bins=/ENTRY:mainCRTStartup");
            
            // 如果有 .rc 文件，可以在这里处理
            let rc_path = Path::new("resources/app.rc");
            if rc_path.exists() {
                println!("cargo:rerun-if-changed=resources/app.rc");
                // 这里可以添加资源编译逻辑
            }
        }
    }
    
    // 监听图标文件变化
    println!("cargo:rerun-if-changed=resources/icons/icon.svg");
    println!("cargo:rerun-if-changed=resources/icons/icon.ico");
    println!("cargo:rerun-if-changed=resources/icons/icon.icns");
}
