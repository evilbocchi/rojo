# This script builds Rojo for WebAssembly using wasm-pack.
# It temporarily modifies Cargo.toml to add "cdylib" crate-type,
# which is required by wasm-pack but causes linker collisions on Windows.

$ErrorActionPreference = "Stop"

# Ensure we are in the project root
$ProjectRoot = Resolve-Path "$PSScriptRoot\.."
Push-Location $ProjectRoot

# Backup Cargo.toml
Copy-Item Cargo.toml Cargo.toml.bak

try {
    # Add cdylib to crate-type
    $Content = Get-Content Cargo.toml -Raw
    $Content = $Content -replace 'crate-type = \["rlib"\]', 'crate-type = ["rlib", "cdylib"]'
    Set-Content Cargo.toml $Content -NoNewline

    # Build using wasm-pack
    wasm-pack build --target bundler --out-name rojo @args
}
finally {
    # Restore Cargo.toml
    Move-Item Cargo.toml.bak Cargo.toml -Force
    Pop-Location
}

Write-Host "WASM build complete! Output is in the 'pkg' directory." -ForegroundColor Green
