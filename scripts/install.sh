#!/usr/bin/env sh
set -eu

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) target="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64|Linux-arm64) target="aarch64-unknown-linux-gnu" ;;
  Darwin-arm64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64) target="x86_64-apple-darwin" ;;
  *) echo "Unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

destination="${WMW_INSTALL_DIR:-$HOME/.local/bin}"
archive="${TMPDIR:-/tmp}/wmw-$target.zip"
mkdir -p "$destination"
curl -fsSL "https://github.com/lucasrgt/wake-me-when/releases/latest/download/wmw-$target.zip" -o "$archive"
unzip -jo "$archive" "*/wmw" -d "$destination"
chmod +x "$destination/wmw"
rm -f "$archive"
echo "Installed wmw to $destination/wmw"
