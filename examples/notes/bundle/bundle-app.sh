#!/bin/sh
# Builds examples/target/Notes.app, so the menu bar shows "Notes".

set -eu

examples_root=$(cd "$(dirname "$0")/../.." && pwd)
app_path="$examples_root/target/Notes.app"

cargo build --release --manifest-path "$examples_root/Cargo.toml" --bin notes

mkdir -p "$app_path/Contents/MacOS"
cp "$examples_root/target/release/notes" "$app_path/Contents/MacOS/Notes"
cp "$examples_root/notes/bundle/Info.plist" "$app_path/Contents/Info.plist"

codesign --force --sign - --timestamp=none "$app_path"
echo "$app_path"
