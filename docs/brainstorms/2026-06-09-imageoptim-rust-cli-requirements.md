---
date: 2026-06-09
topic: imageoptim-rust-cli
---

# imageoptim-rs — Rust CLI Image Optimizer

## Problem Frame

Inspired by `JamieMason/ImageOptim-CLI` (3.5k stars, archived 2023-11), build a CLI tool for batch image optimization that is part of automated build processes. The original is a macOS-only AppleScript orchestrator that shells out to GUI applications. The Rust version uses native Rust crates directly, runs cross-platform, and ships as a single static binary.

Primary user: developers integrating image optimization into build pipelines (web projects, static sites, asset pipelines) who want a no-runtime, no-GUI, no-dependency tool.

## Requirements

### Functional

- **R1.** Single command `imageoptim` accepts one or more file paths or glob patterns as positional arguments.
- **R2.** Auto-detects format by file extension and routes to the appropriate Rust crate:
  - `.png` → `oxipng`
  - `.jpg` / `.jpeg` → `mozjpeg` (or `jpeg-encoder` if mozjpeg build is impractical)
  - `.gif` → `gif` crate (re-encode with frame optimization)
  - `.webp` → `webp` crate (lossless re-encode)
  - `.svg` → `usvg` (path simplification, attribute cleanup)
- **R3.** Default behavior is **safe in-place overwrite**: only writes the optimized file back when (a) the optimized output is strictly smaller than the original, AND (b) the optimized file decodes successfully (validated by re-encoding the first frame / first scanline).
- **R4.** Supports recursive directory traversal with `-r` / `--recursive`.
- **R5.** Supports a glob pattern syntax compatible with shell globs (`*.png`, `**/*.jpg`).
- **R6.** Reports per-file statistics: original size, optimized size, savings in bytes and percent.
- **R7.** Aggregates summary at the end: total files processed, total bytes saved, total percent saved.
- **R8.** Exit code: `0` if all files processed successfully, `1` if any file failed (with errors streamed to stderr).

### Flags

- `--dry-run` — show what would be done, do not modify any files.
- `-r`, `--recursive` — recurse into directories.
- `-q`, `--quality <0-100>` — quality for lossy formats (default: lossless where possible).
- `--no-color` — disable ANSI color in output.
- `-j`, `--jobs <n>` — number of parallel workers (default: number of logical CPUs).
- `-V`, `--version` — print version.
- `-h`, `--help` — print usage.

## Success Criteria

- A developer can install the binary, run `imageoptim 'assets/**/*.png' -r`, and observe a non-zero total savings in the summary.
- A file that fails to optimize (e.g., oxipng errors on a corrupt PNG) does not overwrite the original — verified by running on a deliberately corrupted fixture.
- The CLI works on macOS, Linux, and Windows with no system-level dependencies beyond the standard C runtime.
- The binary is self-contained (statically linkable) for `cargo install` and direct download.
- Unit + integration tests cover: format detection, the "only write if smaller AND valid" safety contract, glob expansion, recursive directory traversal, and the summary aggregation.

## Scope Boundaries

- **In scope:** lossless optimization for all five formats; lossy JPEG/WebP re-encoding behind a quality flag; safe in-place overwrite.
- **Out of scope (deferred):**
  - Cross-format conversion (PNG → WebP). Not in original CLI; format-changing is a different product surface.
  - Animation handling beyond what the Rust crates provide out of the box.
  - EXIF / metadata preservation policy. Default: strip metadata (matches `oxipng` default; matches ImageOptim-CLI default).
  - Web UI / TUI / GUI.
  - Network / cloud optimization.
  - Watching directories for changes (no `--watch` flag).

## Key Decisions

- **Rust native, not shell-out.** The original's `oxipng` / `mozjpeg` / `gifsicle` are CLI tools, but Rust has mature crate bindings for all of them. Direct use avoids the "tool not installed" failure mode and gives a single-binary distribution.
- **Safe in-place default.** A bad write would silently corrupt a user's asset directory. The "only write if strictly smaller AND decodes valid" rule makes the default state the safe one.
- **Format auto-detection by extension.** Matches user mental model and ImageOptim-CLI behavior. No `--format` flag needed for the common case.
- **Reuse, don't reinvent.** Lean on the existing Rust ecosystem: `oxipng`, `mozjpeg`, `gif`, `webp`, `usvg`. No custom PNG/JPEG encoder.

## Dependencies / Assumptions

- The Rust crates `oxipng`, `mozjpeg`, `gif`, `webp`, `usvg` are maintained and API-stable enough to depend on for a 1.0 release.
- `mozjpeg` crate builds cleanly across the three target platforms. **Verify in planning** — mozjpeg has a C dependency and can be finicky to cross-compile. Fallback: `jpeg-encoder` (pure Rust, lower compression ratio).
- Target users have files on local filesystem accessible to the binary. No NFS / FUSE edge cases.
- CLI parsing via `clap` (de facto standard).

## Outstanding Questions

### Resolve Before Planning

- *(none — all blocking product decisions resolved)*

### Deferred to Planning

- [Affects R2][Technical] `mozjpeg` build feasibility on Windows and musl targets — likely needs fallback to pure-Rust JPEG encoder. Planner should pick the JPEG crate.
- [Affects R3][Technical] Exact "decode-valid" check per format. For PNG: re-decode with `png` crate and compare dimensions. For GIF: re-decode first frame. For JPEG: re-decode with `jpeg-decoder`. Planner should codify the per-format contract.
- [Affects R2][Technical] SVG optimization is more nuanced than the others — `usvg` is a parser/normalizer, not a minifier. Planner should decide whether to wrap `usvg` (canonicalization) or add `svgcleaner` / `svgo-rs` for true minification.
- [Affects R6][Technical] Output format: human-readable table vs. JSON. Planner should pick based on `stdout` vs `--json` flag decision.

## Next Steps

→ `/ce:plan` for structured implementation planning.
