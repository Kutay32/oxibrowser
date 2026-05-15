#!/bin/bash
# OxiBrowser macOS .app bundle oluşturma scripti
# Kullanım: ./scripts/bundle_macos.sh [release|debug]

set -euo pipefail

PROFILE="${1:-release}"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="OxiBrowser"
BUNDLE_ID="com.oxibrowser.app"
BUILD_DIR="$PROJECT_ROOT/target/$PROFILE"

echo "==> OxiBrowser macOS Bundle Oluşturucu"
echo "Profil: $PROFILE"

# 1. Binary'yi build et
echo "==> Binary derleniyor..."
if [ "$PROFILE" = "release" ]; then
    cargo build --release --features desktop
else
    cargo build --features desktop
fi

# 2. .app bundle yapısını oluştur
APP_BUNDLE="$PROJECT_ROOT/target/$APP_NAME.app"
echo "==> .app bundle oluşturuluyor: $APP_BUNDLE"

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# 3. Binary'yi kopyala
cp "$BUILD_DIR/oxibrowser" "$APP_BUNDLE/Contents/MacOS/"

# 4. Info.plist'i kopyala
if [ -f "$PROJECT_ROOT/assets/Info.plist" ]; then
    cp "$PROJECT_ROOT/assets/Info.plist" "$APP_BUNDLE/Contents/"
else
    echo "UYARI: assets/Info.plist bulunamadı."
fi

# 5. Entitlements dosyasını kopyala
if [ -f "$PROJECT_ROOT/assets/oxibrowser.entitlements" ]; then
    cp "$PROJECT_ROOT/assets/oxibrowser.entitlements" "$APP_BUNDLE/Contents/"
fi

# 6. App icon'u kopyala (varsa)
if [ -f "$PROJECT_ROOT/assets/AppIcon.icns" ]; then
    cp "$PROJECT_ROOT/assets/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/"
elif [ -d "$PROJECT_ROOT/assets/AppIcon.iconset" ]; then
    iconutil -c icns "$PROJECT_ROOT/assets/AppIcon.iconset" -o "$APP_BUNDLE/Contents/Resources/AppIcon.icns"
fi

# 7. Code sign (geliştirici sertifikası varsa)
if command -v codesign &> /dev/null; then
    echo "==> Code signing uygulanıyor..."
    codesign --force --deep --sign - "$APP_BUNDLE" 2>/dev/null || true
fi

echo ""
echo "==> Başarılı! .app bundle oluşturuldu:"
echo "    $APP_BUNDLE"
echo ""
echo "Çalıştırmak için: open $APP_BUNDLE"
echo ""
echo "Not: İmzalama olmadan çalıştırmak için:"
echo "    xattr -dr com.apple.quarantine $APP_BUNDLE"
