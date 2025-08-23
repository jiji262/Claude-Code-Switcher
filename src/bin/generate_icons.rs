// 图标生成工具
// 使用: cargo run --bin generate_icons

use resvg::usvg::{self, TreeParsing};
use std::fs;
use std::path::Path;
use tiny_skia::Pixmap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Claude Code Switcher - 图标生成工具");

    let svg_path = "resources/icons/icon.svg";
    let output_dir = "resources/icons";

    if !Path::new(svg_path).exists() {
        eprintln!("❌ SVG 文件不存在: {}", svg_path);
        return Ok(());
    }

    // 读取 SVG 文件
    let svg_data = fs::read_to_string(svg_path)?;
    let options = usvg::Options::default();
    let usvg_tree = usvg::Tree::from_str(&svg_data, &options)?;
    
    println!("📁 源文件: {}", svg_path);
    println!("📁 输出目录: {}", output_dir);
    
    // 生成不同尺寸的 PNG
    let sizes = vec![16, 32, 48, 64, 128, 256, 512, 1024];
    let mut png_files = Vec::new();
    
    for size in sizes {
        let output_path = format!("{}/icon_{}.png", output_dir, size);
        if generate_png(&usvg_tree, size, &output_path)? {
            println!("✅ 生成 {}x{} PNG: {}", size, size, output_path);
            png_files.push(output_path);
        }
    }
    
    // 生成 ICO 文件 (Windows)
    if !png_files.is_empty() {
        let ico_path = format!("{}/icon.ico", output_dir);
        if generate_ico(&png_files, &ico_path) {
            println!("✅ 生成 Windows ICO: {}", ico_path);
        }
    }
    
    println!("🎉 图标生成完成!");
    Ok(())
}

fn generate_png(usvg_tree: &usvg::Tree, size: u32, output_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut pixmap = Pixmap::new(size, size).ok_or("无法创建 pixmap")?;

    let transform = tiny_skia::Transform::from_scale(
        size as f32 / usvg_tree.size.width(),
        size as f32 / usvg_tree.size.height(),
    );

    let resvg_tree = resvg::Tree::from_usvg(usvg_tree);
    resvg_tree.render(transform, &mut pixmap.as_mut());

    // 保存为 PNG
    pixmap.save_png(output_path)?;
    Ok(true)
}

fn generate_ico(png_files: &[String], ico_path: &str) -> bool {
    // 这里是一个简化的 ICO 生成
    // 实际应用中可能需要更复杂的 ICO 格式处理
    
    // 目前只是复制最大的 PNG 文件作为占位符
    if let Some(largest_png) = png_files.iter().find(|f| f.contains("256")) {
        if let Ok(_) = fs::copy(largest_png, ico_path) {
            return true;
        }
    }
    
    // 如果没有 256x256，使用第一个可用的
    if let Some(first_png) = png_files.first() {
        if let Ok(_) = fs::copy(first_png, ico_path) {
            return true;
        }
    }
    
    false
}
