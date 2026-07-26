#!/usr/bin/env sh
set -eu

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) target="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64|Linux-arm64) target="aarch64-unknown-linux-gnu" ;;
  Darwin-arm64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64) target="x86_64-apple-darwin" ;;
  *) echo "Unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

destination="${NOTYET_INSTALL_DIR:-$HOME/.local/bin}"
archive="${TMPDIR:-/tmp}/notyet-$target.zip"
mkdir -p "$destination"
curl -fsSL "https://github.com/lucasrgt/not-yet/releases/latest/download/notyet-$target.zip" -o "$archive"
unzip -jo "$archive" "*/notyet" -d "$destination"
chmod +x "$destination/notyet"
rm -f "$archive"
echo "Installed notyet to $destination/notyet"
