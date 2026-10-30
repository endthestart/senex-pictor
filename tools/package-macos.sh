#!/bin/sh
# Builds dist/Pictor.app (universal: Apple Silicon and Intel, ad hoc signed) and
# dist/Pictor-macos.zip. Needs rustup's stable toolchain with the aarch64-apple-darwin and
# x86_64-apple-darwin targets, and Xcode's command-line tools (lipo, codesign, ditto).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
toolchain="$HOME/.rustup/toolchains/stable-$(uname -m | sed 's/arm64/aarch64/')-apple-darwin/bin"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/app/Cargo.toml" | head -1)
targets="aarch64-apple-darwin x86_64-apple-darwin"

cd "$root"
for t in $targets; do
    RUSTC="$toolchain/rustc" CARGO_TARGET_DIR=target/package \
        "$toolchain/cargo" build --release -q -p pictor --target "$t"
done

app=dist/Pictor.app
rm -rf "$app" dist/Pictor-macos.zip
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/pictor" \
    $(for t in $targets; do echo "target/package/$t/release/pictor"; done)
strip -S "$app/Contents/MacOS/pictor"
cp app/assets/icon/pictor.icns "$app/Contents/Resources/pictor.icns"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Pictor</string>
  <key>CFBundleDisplayName</key><string>Pictor</string>
  <key>CFBundleIdentifier</key><string>com.senex.pictor</string>
  <key>CFBundleExecutable</key><string>pictor</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleIconFile</key><string>pictor</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.photography</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>SENEX Pictor · MIT OR Apache-2.0</string>
</dict>
</plist>
EOF
codesign --force --deep --sign - "$app"
ditto -c -k --keepParent "$app" dist/Pictor-macos.zip
lipo -info "$app/Contents/MacOS/pictor"
ls -lh dist/Pictor-macos.zip
