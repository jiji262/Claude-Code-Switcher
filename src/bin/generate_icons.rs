// 跨平台图标生成工具 (PNG + 标准多尺寸 Windows ICO 容器)
// 使用: cargo run --bin generate_icons

use resvg::usvg::{self, TreeParsing};
use std::fs;
use std::path::Path;
use tiny_skia::Pixmap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Claude Code Switcher - Icon Generator");

    let svg_path = "resources/icons/icon.svg";
    let output_dir = "resources/icons";

    if !Path::new(svg_path).exists() {
        eprintln!("Error: SVG source not found at {}", svg_path);
        return Ok(());
    }

    let svg_data = fs::read_to_string(svg_path)?;
    let options = usvg::Options::default();
    let usvg_tree = usvg::Tree::from_str(&svg_data, &options)?;

    let sizes = [16u32, 32, 48, 64, 128, 256, 512, 1024];
    for size in sizes {
        let output_path = format!("{}/icon_{}.png", output_dir, size);
        generate_png(&usvg_tree, size, &output_path)?;
        println!("Generated {}x{} PNG -> {}", size, size, output_path);
    }

    // Write genuine multi-resolution Microsoft ICO container (16, 32, 48, 256 PNG frames)
    let ico_sizes = [16u32, 32, 48, 256];
    let ico_path = format!("{}/icon.ico", output_dir);
    write_multi_size_ico(output_dir, &ico_sizes, &ico_path)?;
    println!("Generated multi-resolution Windows ICO -> {}", ico_path);

    Ok(())
}

fn generate_png(
    usvg_tree: &usvg::Tree,
    size: u32,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pixmap = Pixmap::new(size, size).ok_or("Failed to allocate pixmap")?;
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / usvg_tree.size.width(),
        size as f32 / usvg_tree.size.height(),
    );
    let resvg_tree = resvg::Tree::from_usvg(usvg_tree);
    resvg_tree.render(transform, &mut pixmap.as_mut());
    pixmap.save_png(output_path)?;
    Ok(())
}

fn write_multi_size_ico(
    output_dir: &str,
    sizes: &[u32],
    ico_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut frames: Vec<(u32, Vec<u8>)> = Vec::with_capacity(sizes.len());
    for &sz in sizes {
        let png_path = format!("{}/icon_{}.png", output_dir, sz);
        let data = fs::read(png_path)?;
        frames.push((sz, data));
    }

    let count = frames.len() as u16;
    let header_len = 6usize + (frames.len() * 16);
    let total_payload: usize = frames.iter().map(|(_, b)| b.len()).sum();
    let mut out = Vec::with_capacity(header_len + total_payload);

    // ICONDIR (6 bytes): reserved=0, type=1 (icon), count
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());

    // ICONDIRENTRY (16 bytes per frame)
    let mut offset = header_len as u32;
    for (sz, bytes) in &frames {
        let dim_byte = if *sz >= 256 { 0u8 } else { *sz as u8 };
        let byte_len = bytes.len() as u32;
        out.push(dim_byte); // bWidth (0 means 256)
        out.push(dim_byte); // bHeight (0 means 256)
        out.push(0); // bColorCount
        out.push(0); // bReserved
        out.extend_from_slice(&1u16.to_le_bytes()); // wPlanes
        out.extend_from_slice(&32u16.to_le_bytes()); // wBitCount (32-bit RGBA)
        out.extend_from_slice(&byte_len.to_le_bytes()); // dwBytesInRes
        out.extend_from_slice(&offset.to_le_bytes()); // dwImageOffset
        offset += byte_len;
    }

    for (_, bytes) in &frames {
        out.extend_from_slice(bytes);
    }

    fs::write(ico_path, out)?;
    Ok(())
}
