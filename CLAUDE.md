# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build              # Build (debug)
cargo build --release    # Build (release)
cargo run                # Run the application
RUST_LOG=debug cargo run # Run with debug logging
cargo test               # Run all tests
cargo test <name>        # Run a single test
cargo clippy             # Lint
cargo fmt                # Format
```

### Dependencies

- **gpui-component** is a local path dependency at `../gpui-component/crates/ui` — it must be cloned alongside this repo.
- **rawler** is pulled from the `trawls-app/dnglab` GitHub fork.

## Architecture

Trawls-GPUI is a desktop application for merging astronomical startrail photographs. Users load RAW lightframes (and optional darkframes for noise reduction), process them with a parallel merge pipeline, and output a DNG file with JPEG preview.

### Two-layer design: `ui/` and `processing/`

**`processing/`** — Image processing, decoupled from the UI:
- `interface.rs`: `FrameIdentifier` — wrapper around file paths for frame identification.
- `status.rs`: `MetadataStatus` and `ProcessingStatus` callback traits. The UI implements these to receive progress updates without the processing layer depending on UI types.
- `metadata.rs`: `FrameMetadataLoader` trait for EXIF extraction. Concrete impl uses `rawler`.
- `image.rs`: `Frame` and `Image` structs. `Mergable` trait with two modes: `Maximize` (pixel max for stacking) and `WeightedAverage` (for darkframes).
- `merge.rs`: `run_merge()` — Rayon parallel pipeline. Loads frames in parallel, reduces via sequential merge. Status updates through `Arc<Mutex<impl ProcessingStatus>>`.
- `dng_writing.rs`: `ImageWriter` — writes TIFF-based DNG output with EXIF data and JPEG preview.

**`ui/`** — GPUI interface (uses Zed's `gpui` crate + `gpui-component`):
- `app/mod.rs`: `Trawls` — single state root for the entire application. Holds lightframes, darkframes, processing counters, preview, and UI state. Mutations call `cx.notify()` to trigger re-renders.
- `app/logic.rs`: Event handlers — file picking, async metadata loading, processing dispatch, preview zoom/pan, statistics computation. Implements `MetadataStatus` and `ProcessingStatus` traits.
- `app/views/`: Rendering — `shell.rs` (sidebar + content layout), `steps.rs` (4-step workflow: lightframes → darkframes → processing → preview), `table.rs` (virtual-scrolling frame table).
- `model.rs`: Shared data types (`FrameInfo`, `MergeMode`, `PreviewData`, `LightframeStats`).

### Key patterns

- **Trait-based decoupling**: Processing communicates back to the UI exclusively through `MetadataStatus` and `ProcessingStatus` traits, never importing UI types directly.
- **Thread communication**: Processing runs on a separate thread. Status updates flow through `Arc<Mutex<>>` and are polled/forwarded to the GPUI entity via `cx.spawn()`.
- **GPUI entity model**: `Trawls` is created as a GPUI entity (`cx.new()`). Views read its fields and render; mutations go through methods that take `&mut self` + context.
