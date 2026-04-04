# Trawls
<img align="right" width="128" height="128" src="assets/images/trawls-logo.png">

[![CI](https://github.com/trawls-app/trawls/actions/workflows/lint.yml/badge.svg)](https://github.com/trawls-app/trawls/actions/workflows/lint.yml)
[![CI](https://github.com/trawls-app/trawls/actions/workflows/test-on-pr.yml/badge.svg)](https://github.com/trawls-app/trawls/actions/workflows/test-on-pr.yml)

Trawls is a fast image processing tool to merge nightsky RAW photos to create a startrail image and output it as another RAW file.

Users load RAW lightframes (and optional darkframes for noise reduction), process them with a parallel merge pipeline, and output a DNG file with JPEG preview.

## Features

- Load and merge RAW lightframes into startrail composites
- Optional darkframe subtraction for noise reduction
- Multiple merge modes: Normal, Falling Comets, Raising Comets
- Parallel processing pipeline using Rayon
- DNG output with embedded JPEG preview and EXIF metadata

## Download

Pre-compiled packages can be found on the [release page](https://github.com/trawls-app/trawls/releases).

## Building from source

Trawls is a native Rust desktop application built with [GPUI](https://gpui.rs/) (Zed's GPU-accelerated UI framework) and [gpui-component](https://github.com/longbridge/gpui-component).

### Prerequisites

- Rust (stable toolchain)
- Linux system dependencies:
  ```
  sudo apt-get install -y libvulkan1 libwayland-dev libxkbcommon-x11-dev libfontconfig-dev libssl-dev libgit2-dev cmake
  ```

### Build and run

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Run the application
RUST_LOG=debug cargo run # Run with debug logging
```

### Linting and testing

```bash
cargo clippy             # Lint
cargo fmt                # Format
cargo test               # Run all tests
```

## Architecture

The codebase is split into two layers:

- **`src/processing/`** — Image processing logic, fully decoupled from the UI. Handles RAW frame loading, pixel-level merge operations, and DNG output writing.
- **`src/ui/`** — GPUI-based desktop interface with a 4-step workflow: lightframes, darkframes, processing, and preview.

Processing communicates back to the UI through callback traits (`MetadataStatus`, `ProcessingStatus`), keeping the two layers independent.
