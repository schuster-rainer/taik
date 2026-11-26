#!/bin/bash
set -euo pipefail

source_dir=$(cd "$(dirname "$0")" && pwd)
install_dir="$HOME/Applications"
app="$install_dir/TAIK Factory.app"
identifier=local.taik.factory.view
mkdir -p "$install_dir"

if [ -e "$app" ]; then
  previous=$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$app/Contents/Info.plist")
  if [ "$previous" != "$identifier" ]; then
    printf '%s\n' "An unrelated app already exists at $app" >&2
    exit 1
  fi
fi

staging=$(mktemp -d "$install_dir/.taik-factory-view.XXXXXX")
trap 'rmdir "$staging" 2>/dev/null || true' EXIT
bundle="$staging/TAIK Factory.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
/usr/bin/swiftc -O -warnings-as-errors "$source_dir/factory-view.swift" -o "$bundle/Contents/MacOS/factory-view"
cp "$source_dir/factory-view.html" "$bundle/Contents/Resources/factory-view.html"
icon_source="$source_dir/../assets/taik-mark-dark.png"
if [ -f "$icon_source" ]; then
  iconset="$staging/TAIK.iconset"
  mkdir "$iconset"
  for size in 16 32 128 256 512; do
    /usr/bin/sips -z "$size" "$size" "$icon_source" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    /usr/bin/sips -z "$((size * 2))" "$((size * 2))" "$icon_source" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  /usr/bin/iconutil -c icns "$iconset" -o "$bundle/Contents/Resources/TAIK.icns"
  rm -rf "$iconset"
fi
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>TAIK Factory</string>
  <key>CFBundleIdentifier</key><string>local.taik.factory.view</string>
  <key>CFBundleExecutable</key><string>factory-view</string>
  <key>CFBundleIconFile</key><string>TAIK</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

# The gate of the app: the freshly built host must hand the viewer well-formed data, and the viewer
# must pass its contract with it. Otherwise the installed app stays as it is. The decision is recorded.
records="$source_dir/../evidence/app"
mkdir -p "$records"
number=1
while [ -e "$records/$(printf 'release-%03d.run' "$number")" ]; do number=$((number + 1)); done
record="$records/$(printf 'release-%03d.run' "$number")"
snapshot="$staging/snapshot.html"
"$bundle/Contents/MacOS/factory-view" --snapshot > "$snapshot"
data=$(node "$source_dir/check-app.mjs" "$bundle/Contents/Resources/factory-view.html" < "$snapshot" 2>&1 | grep -E '^PASS|Assertion' | head -1 || true)
view=$(/usr/bin/swift "$source_dir/check-factory-design.swift" --require-open < "$snapshot" 2>&1 | tail -1 || true)
rm -f "$snapshot"
case "$data$view" in
  PASS*PASS*) decision=accepted ;;
  *) decision=rejected ;;
esac
printf 'decision=%s\nkind=release\nidentity=app-release-%03d\nsource_sha256=%s\nviewer_sha256=%s\nreleased_by=tools/open-factory-view.sh\nchecks=data: %s viewer: %s\nend\n' \
  "$decision" "$number" "$(shasum -a 256 "$source_dir/factory-view.swift" | cut -d' ' -f1)" \
  "$(shasum -a 256 "$source_dir/factory-view.html" | cut -d' ' -f1)" "$data" "$view" > "$record"
if [ "$decision" != accepted ]; then
  cat "$record" >&2
  rm -rf "$bundle"
  exit 1
fi

if [ -d "$app" ]; then
  mv "$app" "$staging/previous.app"
fi
if ! mv "$bundle" "$app"; then
  if [ -d "$staging/previous.app" ]; then mv "$staging/previous.app" "$app"; fi
  exit 1
fi
if [ -d "$staging/previous.app" ]; then
  rm -rf "$staging/previous.app"
fi
/usr/bin/open "$app"
