#!/usr/bin/env python3
"""
图标生成脚本
从 SVG 文件生成不同平台所需的图标格式
"""

import os
import sys
import subprocess
from pathlib import Path

def check_dependencies():
    """检查必要的依赖"""
    try:
        import cairosvg
        from PIL import Image
    except ImportError as e:
        print(f"❌ 缺少依赖: {e}")
        print("请安装依赖: pip install cairosvg pillow")
        return False
    return True

def svg_to_png(svg_path, png_path, size):
    """将 SVG 转换为指定尺寸的 PNG"""
    try:
        import cairosvg
        cairosvg.svg2png(
            url=str(svg_path),
            write_to=str(png_path),
            output_width=size,
            output_height=size
        )
        return True
    except Exception as e:
        print(f"❌ SVG 转 PNG 失败: {e}")
        return False

def create_ico(png_paths, ico_path):
    """从多个 PNG 文件创建 ICO 文件"""
    try:
        from PIL import Image
        images = []
        for png_path in png_paths:
            if os.path.exists(png_path):
                img = Image.open(png_path)
                images.append(img)
        
        if images:
            images[0].save(ico_path, format='ICO', sizes=[(img.width, img.height) for img in images])
            return True
    except Exception as e:
        print(f"❌ 创建 ICO 失败: {e}")
    return False

def create_icns(png_path, icns_path):
    """创建 macOS ICNS 文件"""
    try:
        # 使用 iconutil (macOS 系统工具)
        iconset_dir = icns_path.with_suffix('.iconset')
        os.makedirs(iconset_dir, exist_ok=True)
        
        # 生成不同尺寸的图标
        sizes = [16, 32, 64, 128, 256, 512, 1024]
        from PIL import Image
        
        base_img = Image.open(png_path)
        
        for size in sizes:
            # 标准分辨率
            resized = base_img.resize((size, size), Image.Resampling.LANCZOS)
            resized.save(iconset_dir / f"icon_{size}x{size}.png")
            
            # 高分辨率 (2x)
            if size <= 512:
                resized_2x = base_img.resize((size * 2, size * 2), Image.Resampling.LANCZOS)
                resized_2x.save(iconset_dir / f"icon_{size}x{size}@2x.png")
        
        # 使用 iconutil 生成 icns
        result = subprocess.run([
            'iconutil', '-c', 'icns', str(iconset_dir), '-o', str(icns_path)
        ], capture_output=True, text=True)
        
        if result.returncode == 0:
            # 清理临时文件
            import shutil
            shutil.rmtree(iconset_dir)
            return True
        else:
            print(f"❌ iconutil 失败: {result.stderr}")
    except Exception as e:
        print(f"❌ 创建 ICNS 失败: {e}")
    return False

def main():
    if not check_dependencies():
        return 1
    
    # 路径设置
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    svg_path = project_root / "resources" / "icons" / "icon.svg"
    output_dir = project_root / "resources" / "icons"
    
    if not svg_path.exists():
        print(f"❌ SVG 文件不存在: {svg_path}")
        return 1
    
    print("🎨 开始生成图标...")
    print(f"📁 源文件: {svg_path}")
    print(f"📁 输出目录: {output_dir}")
    
    # 创建临时 PNG 文件
    temp_pngs = []
    sizes = [16, 32, 48, 64, 128, 256]
    
    for size in sizes:
        png_path = output_dir / f"icon_{size}.png"
        if svg_to_png(svg_path, png_path, size):
            temp_pngs.append(png_path)
            print(f"✅ 生成 {size}x{size} PNG")
        else:
            print(f"❌ 生成 {size}x{size} PNG 失败")
    
    # 生成 ICO 文件 (Windows)
    ico_path = output_dir / "icon.ico"
    if create_ico(temp_pngs, ico_path):
        print(f"✅ 生成 Windows ICO: {ico_path}")
    else:
        print("❌ 生成 Windows ICO 失败")
    
    # 生成 ICNS 文件 (macOS)
    if sys.platform == "darwin" and temp_pngs:
        icns_path = output_dir / "icon.icns"
        # 使用最大的 PNG 作为基础
        largest_png = max(temp_pngs, key=lambda p: int(p.stem.split('_')[1]))
        if create_icns(largest_png, icns_path):
            print(f"✅ 生成 macOS ICNS: {icns_path}")
        else:
            print("❌ 生成 macOS ICNS 失败")
    
    # 清理临时文件
    for png_path in temp_pngs:
        try:
            os.remove(png_path)
        except:
            pass
    
    print("🎉 图标生成完成!")
    return 0

if __name__ == "__main__":
    sys.exit(main())
