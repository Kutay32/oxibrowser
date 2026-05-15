#!/usr/bin/env python3
"""
OxiBrowser App Icon Generator
-macOS: .icns / .iconset
-iOS:   .xcassets

Kullanım: python3 scripts/generate_icons.py [--macos | --ios | --all]

Requires: pip install Pillow
"""

import subprocess
import sys
import os
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
ASSETS_DIR = PROJECT_ROOT / "assets"

MACOS_ICONSET = ASSETS_DIR / "AppIcon.iconset"
MACOS_ICNS = ASSETS_DIR / "AppIcon.icns"

IOS_APPICON = ASSETS_DIR / "AppIcon.xcassets" / "AppIcon.appiconset"

ICON_SIZES = {
    "macos": [
        (16, 16, "icon_16x16.png"),
        (32, 32, "icon_16x16@2x.png"),
        (32, 32, "icon_32x32.png"),
        (64, 64, "icon_32x32@2x.png"),
        (128, 128, "icon_128x128.png"),
        (256, 256, "icon_128x128@2x.png"),
        (256, 256, "icon_256x256.png"),
        (512, 512, "icon_256x256@2x.png"),
        (512, 512, "icon_512x512.png"),
        (1024, 1024, "icon_512x512@2x.png"),
    ],
    "ios": [
        (40, 40, "icon-20@2x.png", "iphone"),
        (60, 60, "icon-20@3x.png", "iphone"),
        (58, 58, "icon-29@2x.png", "iphone"),
        (87, 87, "icon-29@3x.png", "iphone"),
        (80, 80, "icon-40@2x.png", "iphone"),
        (120, 120, "icon-40@3x.png", "iphone"),
        (120, 120, "icon-60@2x.png", "iphone"),
        (180, 180, "icon-60@3x.png", "iphone"),
        (20, 20, "icon-20~ipad.png", "ipad"),
        (40, 40, "icon-20@2x~ipad.png", "ipad"),
        (29, 29, "icon-29~ipad.png", "ipad"),
        (58, 58, "icon-29@2x~ipad.png", "ipad"),
        (40, 40, "icon-40~ipad.png", "ipad"),
        (80, 80, "icon-40@2x~ipad.png", "ipad"),
        (76, 76, "icon-76~ipad.png", "ipad"),
        (152, 152, "icon-76@2x~ipad.png", "ipad"),
        (167, 167, "icon-83.5@2x~ipad.png", "ipad"),
        (1024, 1024, "icon-1024.png", "ios-marketing"),
    ],
}


def try_import_pillow():
    try:
        from PIL import Image, ImageDraw
        return Image, ImageDraw
    except ImportError:
        print("Pillow gerekli. Yüklemek için: pip install Pillow")
        sys.exit(1)


def create_base_icon(size):
    Image, ImageDraw = try_import_pillow()
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Gradient background
    for y in range(size):
        r = int(30 + (y / size) * 50)
        g = int(100 + (y / size) * 80)
        b = int(200 + (y / size) * 55)
        draw.line([(0, y), (size, y)], fill=(r, g, b, 255))

    # Rounded rectangle overlay
    margin = size // 8
    rect = [margin, margin, size - margin, size - margin]
    corner_radius = size // 6
    draw.rounded_rectangle(rect, radius=corner_radius, fill=(255, 255, 255, 40))

    # "O" letter icon
    font_size = size // 3
    center = size // 2
    r = font_size // 2
    draw.ellipse(
        [center - r, center - r, center + r, center + r],
        outline=(255, 255, 255, 230),
        width=max(2, size // 40),
    )

    return img


def generate_macos_icons():
    print("==> macOS ikonları oluşturuluyor...")

    MACOS_ICONSET.mkdir(parents=True, exist_ok=True)

    for w, h, name in ICON_SIZES["macos"]:
        img = create_base_icon(w)
        path = MACOS_ICONSET / name
        img.save(path, "PNG")
        print(f"    Oluşturuldu: {name} ({w}x{h})")

    # Convert iconset -> icns using iconutil
    print("==> .icns dosyası oluşturuluyor (iconutil)...")
    result = subprocess.run(
        ["iconutil", "-c", "icns", str(MACOS_ICONSET), "-o", str(MACOS_ICNS)],
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        print(f"    Başarılı: {MACOS_ICNS}")
    else:
        print(f"    Hata: {result.stderr}")

    print("==> macOS ikonları tamamlandı.")


def generate_ios_icons():
    print("==> iOS ikonları oluşturuluyor...")

    IOS_APPICON.mkdir(parents=True, exist_ok=True)

    contents = {
        "images": [],
        "info": {"author": "xcode", "version": 1},
    }

    for w, h, name, idiom in ICON_SIZES["ios"]:
        img = create_base_icon(w)
        path = IOS_APPICON / name
        img.save(path, "PNG")
        print(f"    Oluşturuldu: {name} ({w}x{h}, {idiom})")

        scale = "@3x" if "3x" in name else ("@2x" if "@2x" in name else "1x")
        size_str = f"{w // int(scale[-2]) if scale[-2].isdigit() else w}x{h // int(scale[-2]) if scale[-2].isdigit() else h}"

        contents["images"].append(
            {
                "filename": name,
                "idiom": idiom,
                "scale": scale,
                "size": f"{w/100:.1f}x{h/100:.1f}".replace(".0", ""),
            }
        )

    import json
    with open(IOS_APPICON / "Contents.json", "w") as f:
        json.dump(contents, f, indent=2)

    print("==> iOS ikonları tamamlandı.")


def main():
    args = set(sys.argv[1:]) if len(sys.argv) > 1 else {"--all"}
    do_macos = "--macos" in args or "--all" in args
    do_ios = "--ios" in args or "--all" in args

    if do_macos:
        generate_macos_icons()
    if do_ios:
        generate_ios_icons()

    if not do_macos and not do_ios:
        print("Kullanım: python3 scripts/generate_icons.py [--macos | --ios | --all]")


if __name__ == "__main__":
    main()
