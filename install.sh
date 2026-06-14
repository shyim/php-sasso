#!/usr/bin/env bash
# Build php-sasso in release mode and install it into PHP's extension dir.
#
# Usage: ./install.sh
#
# On macOS bindgen needs libclang; we point LIBCLANG_PATH at Homebrew's LLVM
# if it is present and the var is not already set.
set -euo pipefail

cd "$(dirname "$0")"

if [[ "$(uname)" == "Darwin" && -z "${LIBCLANG_PATH:-}" ]]; then
  for cand in /opt/homebrew/opt/llvm/lib /usr/local/opt/llvm/lib; do
    if [[ -f "$cand/libclang.dylib" ]]; then
      export LIBCLANG_PATH="$cand"
      break
    fi
  done
fi

echo ">> cargo build --release"
cargo build --release

EXT_DIR="$(php-config --extension-dir)"
# Loadable PHP modules are referenced as `.so` in `extension=` even on macOS.
case "$(uname)" in
  Darwin) SRC="target/release/libsasso.dylib" ;;
  *)      SRC="target/release/libsasso.so" ;;
esac

DEST="$EXT_DIR/sasso.so"
echo ">> installing $SRC -> $DEST"
# May need sudo depending on where PHP lives.
if [[ -w "$EXT_DIR" ]]; then
  cp "$SRC" "$DEST"
else
  sudo cp "$SRC" "$DEST"
fi

echo
echo "Installed. Enable it by adding this line to your php.ini:"
echo
echo "    extension=sasso.so"
echo
echo "(php.ini path: $(php-config --ini-path 2>/dev/null || php --ini | sed -n 's/.*: //p' | head -1))"
echo "Verify with:  php -m | grep -i sasso   (after enabling)"
