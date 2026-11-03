#!/usr/bin/env bash
# Build Spell Research and install it where the client loads plugins from.
#
#   scripts/build.sh          # build and install
#   scripts/build.sh --test   # run the native unit tests first
#
# The client watches its plugins folder and reloads a plugin a second after
# its module or manifest changes, so installing while a session runs is the
# normal way to test: say `/plugin reload all`, then `/tradebrowse`.
set -euo pipefail
cd "$(dirname "$0")/.."

# cargo lives in the toolchain directory here, not on PATH.
for candidate in \
    "$HOME/.rustup/toolchains/1.95.0-aarch64-apple-darwin/bin" \
    "$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin" \
    "$HOME/.cargo/bin"; do
    if [ -x "$candidate/cargo" ]; then
        PATH="$candidate:$PATH"
        break
    fi
done
command -v cargo > /dev/null || { echo "cargo not found" >&2; exit 1; }

if [ "${1:-}" = "--test" ]; then
    cargo test
fi

cargo build --release --target wasm32-unknown-unknown

case "$(uname -s)" in
    Darwin) plugins="$HOME/Library/Application Support/openac-rust/plugins" ;;
    Linux)  plugins="${XDG_DATA_HOME:-$HOME/.local/share}/openac-rust/plugins" ;;
    *)      echo "unknown platform; install the folder by hand" >&2; exit 1 ;;
esac

dest="$plugins/liamwestray.trade-browse"
mkdir -p "$dest"
cp target/wasm32-unknown-unknown/release/trade_browse.wasm "$dest/"
cp plugin.json "$dest/"

echo
echo "installed: $dest"
echo "in game:   /plugin reload all"
echo "           /tradebrowse"
echo "           /tradebrowse show"
