#!/usr/bin/env bash
# Build the quote-of-the-day sample plugin to wasm32-wasip1 and copy the
# resulting artifact next to plugin.toml so the host's load_from_dir can pick
# it up.
set -euo pipefail
cd "$(dirname "$0")"

rustup target add wasm32-wasip1 2>/dev/null || true

cargo build --release --target wasm32-wasip1

cp target/wasm32-wasip1/release/quote_of_the_day.wasm plugin.wasm
echo "Built: $(pwd)/plugin.wasm"
ls -lh plugin.wasm
