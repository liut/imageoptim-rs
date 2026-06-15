# Changelog

All notable changes to imageoptim-rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-06-15

First tagged release. Cross-platform single-binary image optimizer with
five format optimizers and a safety contract (only overwrite when the
optimized output is strictly smaller AND decodes valid for the source
format).

### Added

- 5 format optimizers: PNG (oxipng), JPEG (jpeg-encoder), GIF (gif crate),
  WebP (webp crate), SVG (usvg canonicalizer)
- PNG `--lossy` palette-quantization path via libimagequant, followed by
  oxipng max-compression and an optional `zopflipng` CLI post-pass
- 14 CLI flags:
  - input/output: `--recursive`, `--jobs`, `--output-dir`, `--fail-fast`
  - lossless/lossy: `--quality`, `--lossy`, `--no-zopfli`,
    `--max-colors <N>` (2..=256, requires `--lossy`),
    `--png-optimization-level <0..=6>` (oxipng preset)
  - safety/IO: `--dry-run`, `--no-color`, `--no-backup`
  - observability: `--verbose`, `--summary-only`
- Atomic in-place writes via temp file + rename; cleanup on rename failure
- Per-file `.bak` backup (opt-out via `--no-backup`); first-run only
- Parallel processing via rayon; configurable worker count
- Progress bar via indicatif (TTY only, suppressed under redirect)
- GPL-3.0-or-later license (the `imagequant` dependency is copyleft)
- Test fixture generator (`cargo run --example gen-fixtures`) for the
  lossy/lossless regression suite; the 2.3 MB synthetic photo is
  git-ignored

### Verified on

- macOS (aarch64, x86_64)
- Linux (x86_64)
- Windows (x86_64)

[Unreleased]: https://github.com/liut/imageoptim-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/liut/imageoptim-rs/releases/tag/v0.1.0
