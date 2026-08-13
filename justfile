# kristal-debug-tools-gui development recipes.

# Host target triple + executable suffix, computed from the toolchain so the
# sidecar recipe also works on ARM hosts. Global vars (recipe-local `:=`
# variables are not interpolable in this just build).
HOST_TRIPLE := `rustc -vV | sed -n 's/^host: //p'`
EXT := if os() == "windows" { ".exe" } else { "" }

# Prepare the tauri externalBin sidecar for this platform.
# tauri-build checks the externalBin paths at compile time, so a placeholder
# must exist before the sidecar itself can be compiled; it gets overwritten
# with the real binary afterwards.
sidecar:
    mkdir -p src-tauri/binaries
    touch src-tauri/binaries/kristal-run-{{HOST_TRIPLE}}{{EXT}}
    cargo build --release --manifest-path src-tauri/Cargo.toml --bin kristal-run
    cp src-tauri/target/release/kristal-run{{EXT}} src-tauri/binaries/kristal-run-{{HOST_TRIPLE}}{{EXT}}

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
