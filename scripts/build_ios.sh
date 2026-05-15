#!/bin/bash
# OxiBrowser iOS build script
# Kullanım: ./scripts/build_ios.sh [release|debug]

set -euo pipefail

PROFILE="${1:-release}"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> OxiBrowser iOS Build"
echo "Profil: $PROFILE"

# iOS için Rust target'larını kontrol et
for target in "aarch64-apple-ios" "aarch64-apple-ios-sim"; do
    if ! rustup target list --installed | grep -q "$target"; then
        echo "==> Eksik target: $target"
        echo "    Yüklemek için: rustup target add $target"
    fi
done

TARGET="aarch64-apple-ios"
if [ "$PROFILE" = "debug" ]; then
    TARGET="aarch64-apple-ios-sim"
fi

echo "==> Hedef: $TARGET"

# Build iOS library
cargo build --target "$TARGET" $( [ "$PROFILE" = "release" ] && echo "--release" ) --features ios-app

echo ""
echo "==> Build başarılı!"
echo ""
echo "iOS simulator'de test etmek için:"
echo "  cargo build --target aarch64-apple-ios-sim --features ios-app"
echo ""
echo "Xcode projesi oluşturmak için (cargo-xcode):"
echo "  cargo install cargo-xcode"
echo "  cargo xcode"
