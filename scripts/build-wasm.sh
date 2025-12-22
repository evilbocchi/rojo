#!/bin/bash
set -e

# This script builds Rojo for WebAssembly using wasm-pack.
# It temporarily modifies Cargo.toml to add "cdylib" crate-type,
# which is required by wasm-pack but causes linker collisions on Windows.

# Ensure we are in the project root
cd "$(dirname "$0")/.."

# Backup Cargo.toml
cp Cargo.toml Cargo.toml.bak

# Add cdylib to crate-type
sed -i 's/crate-type = \["rlib"\]/crate-type = ["rlib", "cdylib"]/' Cargo.toml

# Build using wasm-pack
wasm-pack build --target bundler --out-name rojo "$@"

# Restore Cargo.toml
mv Cargo.toml.bak Cargo.toml

echo "WASM build complete! Output is in the 'pkg' directory."
