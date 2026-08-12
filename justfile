# kristal-debug-tools-gui development recipes.

# Prepare the tauri externalBin sidecar for this platform.
# tauri-build checks the externalBin paths at compile time, so a placeholder
# must exist before the sidecar itself can be compiled; it gets overwritten
# with the real binary afterwards.
sidecar:
    mkdir -p src-tauri/binaries
    touch src-tauri/binaries/kristal-run-{{ if os() == "windows" { "x86_64-pc-windows-msvc" } else { "x86_64-unknown-linux-gnu" } }}
    cargo build --release --manifest-path src-tauri/Cargo.toml --bin kristal-run
    cp src-tauri/target/release/kristal-run{{ if os() == "windows" { ".exe" } else { "" } }} src-tauri/binaries/kristal-run-{{ if os() == "windows" { "x86_64-pc-windows-msvc" } else { "x86_64-unknown-linux-gnu" } }}

# Full release bundle (builds the sidecar first).
build: sidecar
    npm install
    npm run build
    npm run tauri build

# Check the Rust backend only.
check:
    cargo check

# Run the app in dev mode.
dev:
    npm run tauri dev
