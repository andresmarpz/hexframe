#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
app=dist/mac/Greyscreen.app
target_app="$HOME/Applications/Greyscreen.app"

cargo build --release --locked -p greyscreen -p greyscreen-cli

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/greyscreen target/release/greyscreen-cli "$app/Contents/MacOS/"
sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
iconset="$work/Greyscreen.iconset"
mkdir "$iconset"
render() { rsvg-convert -w "$2" -h "$2" "$1" -o "$iconset/$3.png"; }
for size in 16 32 128 256 512; do
    svg=brand/app-icon.svg
    [ "$size" = 16 ] && svg=brand/app-icon-16.svg
    render "$svg" "$size" "icon_${size}x${size}"
    render brand/app-icon.svg "$((size * 2))" "icon_${size}x${size}@2x"
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/Greyscreen.icns"

codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"

osascript -e 'tell application id "dev.greyscreen.app" to quit' >/dev/null 2>&1 || true
sleep 1
pkill -f "$target_app/Contents/MacOS/greyscreen" 2>/dev/null || true
mkdir -p "$HOME/Applications"
rm -rf "$target_app"
cp -R "$app" "$target_app"

echo "Built $app (version $version) and installed $target_app"
echo "Open it with: open ~/Applications/Greyscreen.app"
