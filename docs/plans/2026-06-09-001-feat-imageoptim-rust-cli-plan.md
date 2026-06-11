---
title: Build imageoptim-rs — Rust CLI Image Optimizer
type: feat
status: active
date: 2026-06-09
origin: docs/brainstorms/2026-06-09-imageoptim-rust-cli-requirements.md
---

> **Plan deviations (2026-06-10):**
> 1. **LICENSE is GPL-3.0-or-later, not MIT.** The plan originally specified MIT, but the `--lossy` PNG path depends on `libimagequant` (GPL-3.0). Linking a GPL-3.0-or-later crate forces the whole crate to GPL-3.0-or-later under standard copyleft interpretation. The user accepted this trade-off; the LICENSE file and `Cargo.toml` `license` field both reflect it.
> 2. **JPEG: `jpeg-encoder` (pure Rust) is the v1 implementation, not `mozjpeg`.** The plan listed `mozjpeg` as preferred with `jpeg-encoder` as fallback. Cross-platform build complexity of `mozjpeg` (Windows MSVC, musl) made the pure-Rust fallback the pragmatic default. The compression ratio gap is real but acceptable for v1.
> 3. **Test fixtures are generated, not committed.** The plan called for `tests/fixtures/` with small sample images, ≤ 50 KB total. In practice the regression tests need a *realistic* (multi-megabyte) photo to exercise the lossy pipeline. The committed tree is now fixture-free; `cargo run --example gen-fixtures` writes the photo to `tests/example01.png` on demand, and the file is git-ignored. `tests/*.png` is the global ignore pattern.
> 4. **`--max-colors <N>` shipped as a follow-up flag, not in v1 scope.** The v1 plan listed `--quality` as the only PNG knob; the `imagequant` palette cap is a separate concern and was deferred to `docs/plans/2026-06-11-001-feat-png-palette-max-colors-plan.md`. The field type is `u32` (not `u8`) because `clap::value_parser!(u8).range(2..=256)` would panic — `u8::MAX` is 255 and the range includes 256.
> 5. **`--png-optimization-level <0..=6>` shipped as a follow-up flag.** The v1 plan did not expose the oxipng preset at all; the per-mode defaults (3 for lossless, 6 for the lossy inner step) were hard-coded. The follow-up was a small trait-widening (added a 6th `png_level: Option<u8>` arg to the `Optimizer` trait) plus a single-line branch in each PNG path; the four non-PNG impls take `_png_level` and ignore it. No dedicated plan doc was written — the change is small enough to inline in the commit message.

# Build imageoptim-rs — Rust CLI Image Optimizer

## Overview

A single-binary Rust CLI that optimizes PNG/JPG/GIF/WebP/SVG images via native Rust crates. Inspired by `JamieMason/ImageOptim-CLI` (3.5k stars, archived 2023), but unlike the original macOS-only AppleScript orchestrator, this version is cross-platform, dependency-free at runtime, and ships as one static binary.

## Problem Statement / Motivation

Web developers and build-pipeline authors need a "drop into CI" image optimizer that:
- Works on macOS, Linux, and Windows without installing GUI apps
- Has no Node.js, no Python, no system-package prerequisites
- Won't silently corrupt the asset directory if a single file is malformed

The original ImageOptim-CLI is tied to three macOS GUI apps and hasn't shipped since 2023-11. `oxipng-cli`, `mozjpeg` binaries, etc. exist individually but require the user to script them and decide per-format. There is no clean Rust-native "one tool, all formats, safe-by-default" answer in the ecosystem.

## Proposed Solution

Single `imageoptim` binary. Glob/path → format detection → Rust-crate optimization → safe in-place write (only if strictly smaller AND decoded-valid) → human-readable per-file report → summary.

## Technical Approach

### Architecture

Workspace structure (single binary, no library crate yet — extract later if a `lib` API is needed):

```
imageoptim-rs/
├── Cargo.toml
├── src/
│   ├── main.rs              # clap CLI definition, top-level dispatch
│   ├── cli.rs               # arg parsing
│   ├── pipeline.rs          # orchestration: expand → detect → optimize → write
│   ├── detect.rs            # extension → format enum mapping
│   ├── optimize/
│   │   ├── mod.rs           # Optimizer trait, dispatch
│   │   ├── png.rs           # oxipng wrapper
│   │   ├── jpeg.rs          # mozjpeg wrapper (or jpeg-encoder fallback)
│   │   ├── gif.rs           # gif crate re-encode
│   │   ├── webp.rs          # webp lossless re-encode
│   │   └── svg.rs           # usvg canonicalization
│   ├── safety.rs            # "smaller + decodes-valid" check per format
│   ├── report.rs            # per-file + summary reporter
│   └── error.rs             # unified Error enum, thiserror
├── tests/
│   ├── fixtures/            # small sample images, plus a deliberately corrupted PNG
│   ├── integration.rs       # end-to-end: glob, recursive, dry-run, summary
│   └── safety.rs            # corrupt-file must not overwrite
└── README.md
```

### Crate Selection

| Concern | Crate | Notes |
|---|---|---|
| CLI parsing | `clap` (v4, derive) | de facto standard |
| Glob | `glob` (the `glob` crate, not `globset`) | shell-compatible globs |
| Errors | `thiserror` + `anyhow` | structured in libs, ad-hoc in main |
| PNG | `oxipng` | Pure Rust; lossless; library API exposed |
| JPEG | `mozjpeg` (preferred) → fallback `jpeg-encoder` | **see Risk** below |
| GIF | `gif` | Re-encode with `Encoder::new`; frame-level optimize |
| WebP | `webp` | Lossless re-encode via `Encoder::from_image` |
| SVG | `usvg` | Parse → serialize; canonical form is the "optimization" |
| Parallelism | `rayon` | `par_iter` over the file list |
| Output | `indicatif` (optional) | Progress bar when output is a TTY |

### Implementation Phases

#### Phase 1: Skeleton & CLI (foundation)

- `cargo init` with binary
- `Cargo.toml` with `clap`, `glob`, `rayon`, `thiserror`, `anyhow`
- `src/main.rs` + `src/cli.rs`: `imageoptim [PATTERNS...]` with `--dry-run`, `-r`, `--jobs`, `--no-color`, `-V`, `-h`
- `src/pipeline.rs`: expand globs into `Vec<PathBuf>`, dedupe, sort
- `src/detect.rs`: extension → `Format` enum
- `src/report.rs`: per-file `Result<Stats, Error>` line printer; final summary
- **Verification:** `imageoptim --help` and `imageoptim --version` work; running on a non-matching glob prints a friendly error and exits 1

#### Phase 2: PNG (vertical slice)

- Add `oxipng` dependency
- `src/optimize/png.rs`: read bytes → `oxipng::optimize_from_memory` → return `Vec<u8>` + original size
- `src/safety.rs`: PNG validity check via `png` crate (`Decoder::new(&bytes).read_info()` succeeds)
- `src/pipeline.rs`: write-back path (`write_atomic` via temp + rename)
- `src/optimize/mod.rs`: `Optimizer` trait with `optimize(&self, bytes: &[u8]) -> Result<Vec<u8>>`
- **Verification:** integration test — optimize a known PNG, assert output is smaller, assert corrupted fixture is NOT overwritten

#### Phase 3: JPEG, GIF, WebP, SVG

- JPEG: `mozjpeg` first; if build fails on a target, fall back to `jpeg-encoder`. Safety check: `jpeg-decoder` round-trip.
- GIF: re-encode with `gif::Encoder`. Safety check: `gif::Decoder::new(&bytes)` parses without error.
- WebP: `webp::Encoder::from_lossless` re-encode. Safety check: `webp::Decoder::new(&bytes).decode()` returns `Some`.
- SVG: `usvg::parse` → `usvg::Tree::to_string(&xml)`. Safety check: `usvg::parse` on the result succeeds.
- Each format: dedicated test in `tests/integration.rs`
- **Verification:** a mixed-format fixture directory runs end-to-end, every file in summary has a stat line

#### Phase 4: Recursive, parallelism, output polish

- `-r` flag: walk directories with `walkdir`
- `-j` flag: `rayon::ThreadPoolBuilder` size, then `par_iter` over files
- `--no-color`: gate ANSI escapes on `IsTerminal` + flag
- Progress bar via `indicatif` (only when stderr is a TTY and `--dry-run` is NOT set)
- **Verification:** large fixture directory shows speedup vs sequential; CI test asserts deterministic output regardless of job count

#### Phase 5: Docs & release readiness

- `README.md`: install (`cargo install imageoptim`), quickstart, examples, comparison to original
- `LICENSE`: MIT (matching original)
- `cargo build --release` produces a single binary
- `--version` reads from `CARGO_PKG_VERSION`

## System-Wide Impact

- **Interaction graph:** `main → cli::parse → pipeline::run → (expand → detect → optimize::dispatch → safety::check → write_atomic → report::print)`. No callbacks, no observers, no side-channels.
- **Error propagation:** `thiserror` enum covers `Io`, `Glob`, `FormatUnknown`, `Optimizer`, `DecodeInvalid`, `NotSmaller`. `main` converts to exit code: 0 success, 1 any per-file error (errors printed to stderr, processing continues for sibling files).
- **State lifecycle risks:** The only persistent state mutation is the in-place write. Mitigated by `write_atomic` (write to `path.tmp`, `rename` over original). On `rename` failure, `path.tmp` is left behind — log a warning, do not panic. A future `--clean-tmp` sweep is a nice-to-have, not v1.
- **API surface parity:** Single CLI surface. No library API in v1 (don't expose `Optimizer` trait publicly yet; if a downstream wants it, add `pub lib.rs` later).
- **Integration test scenarios:**
  1. Glob `**/*.png` against a tree — confirms recursive + format routing
  2. Run on a directory containing 1 corrupt PNG + 3 valid PNGs — confirms corrupt file fails, valid files still optimize, exit code 1
  3. Run with `--dry-run` on a real PNG — confirms file mtime is unchanged and no diff in content
  4. Run with no matches — confirms exit code 1 and helpful error
  5. Run with `-j 1` vs `-j 8` on a 50-file fixture — both succeed, results identical (determinism)

## Acceptance Criteria

### Functional

- [ ] `imageoptim 'assets/**/*.png' -r` runs on a sample directory and reports ≥ 1 byte saved
- [ ] Format auto-detection works for all five: `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.svg`
- [ ] Default behavior: file is overwritten only if optimized bytes are strictly smaller AND decode-valid
- [ ] `--dry-run` does not modify any file (mtime check)
- [ ] Corrupt input file: does NOT overwrite, prints error, processing continues for other files
- [ ] `-j` flag changes concurrency; output is deterministic across job counts
- [ ] Exit code: 0 all-success, 1 any-failure

### Non-Functional

- [ ] Single static binary, no runtime dependencies beyond the C standard library
- [ ] Builds on macOS (aarch64, x86_64), Linux (x86_64), Windows (x86_64)
- [ ] `--help` is informative and lists all flags with defaults
- [ ] No panics on any input (use `Result` everywhere on I/O paths)

### Quality Gates

- [ ] `cargo test` passes (unit + integration)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] Test fixtures committed under `tests/fixtures/` (small sample images, ≤ 50 KB total)
- [ ] README has install, quickstart, and example sections

## Success Metrics

- The 5 integration scenarios in "System-Wide Impact" all pass
- A user can `cargo install --path .` and have a working `imageoptim` binary in `< 5 minutes`
- `imageoptim --help` output fits in one terminal screen and answers "what does this do" within 5 seconds of reading

## Dependencies & Risks

- **`mozjpeg` build complexity.** The `mozjpeg` crate is a Rust binding to a C library; on some cross-compile targets (notably Windows MSVC and musl) it requires extra toolchain. **Mitigation:** feature flag `mozjpeg` (default on) vs `jpeg-encoder-fallback` (pure Rust, smaller compression ratio, builds anywhere). If `mozjpeg` fails on any of the three target platforms, document the fallback and recommend the pure-Rust variant.
- **`oxipng` API stability.** `oxipng` v9+ has a stable library API; pin to `^9`. Recheck at v10 release.
- **WebP lossless re-encode may GROW some files** (e.g., already-optimized WebPs). The strict `strictly smaller` rule handles this — the file is left unchanged. This is the intended behavior.
- **SVG optimization is light.** `usvg` is a canonicalizer (parses → re-serializes), not a minifier like SVGO. For v1 this is honest: we don't claim minification. A future v2 could add `svgcleaner` or a Rust port of SVGO.

## Alternatives Considered

- **Shell-out to system CLIs** (the original's approach). Rejected: requires user to install 5+ tools; defeats the "single binary" goal.
- **Cross-format conversion (PNG→WebP, etc.).** Rejected for v1: scope creep; ImageOptim-CLI didn't do it; changes user data in non-lossless ways.
- **Reuse ImageOptim-CLI's AppleScript** for Mac users. Rejected: the user explicitly chose "Rust native".

## Documentation Plan

- `README.md` sections: Title, one-line description, install (`cargo install`), quickstart, all flags with examples, supported formats, comparison to original ImageOptim-CLI, license
- Inline `///` doc comments on every public item in `cli.rs` and `optimize/mod.rs`
- Comment in `safety.rs` explaining WHY the "smaller + valid" rule is the safety contract

## Sources & References

### Origin

- **Origin document:** [docs/brainstorms/2026-06-09-imageoptim-rust-cli-requirements.md](2026-06-09-imageoptim-rust-cli-requirements.md)
- Carried-forward decisions:
  1. Rust native crates (no shell-out)
  2. PNG/JPG/GIF/WebP/SVG in v1
  3. Safe in-place overwrite default
  4. Single `imageoptim` + flags interface
  5. Strip metadata by default (per-crate defaults)

### Internal References

- None (empty project)

### External References

- Original CLI: <https://github.com/JamieMason/ImageOptim-CLI>
- `oxipng` crate: <https://docs.rs/oxipng>
- `mozjpeg` crate: <https://docs.rs/mozjpeg>
- `gif` crate: <https://docs.rs/gif>
- `webp` crate: <https://docs.rs/webp>
- `usvg` crate: <https://docs.rs/usvg>
- `clap` v4: <https://docs.rs/clap>
- `rayon`: <https://docs.rs/rayon>

### Related Work

- (none)

## Next Steps (post-v1)

Items discussed during the 2026-06-10 build but deferred to a follow-up. Captured here so they don't get lost.

### SVG: stronger minifier (DEFERRED — investigated, not worth it)

`usvg` canonicalizes (parses → re-serializes) but does not minify. Empirically, on a 799-byte test SVG with redundant metadata, `usvg` reaches 468 bytes (41%); `svgo` (Node.js) reaches 372 bytes (54%). The 13-percentage-point gap is real but does not justify the trade-offs:

- **Shell-out to `svgcleaner`**: same pattern as the existing `zopflipng` integration, but `svgcleaner` is less commonly installed than `zopflipng` and the upstream repo has been quiet since 2021. The `svgcleaner` distribution is also a non-trivial Rust CLI build (`cargo install svgcleaner` requires the toolchain).
- **`svgo-rs` crate**: license is GPL-3.0-or-later, same as our `imagequant` choice — would lock the crate to GPL (already locked). Pure-Rust would be the only benefit. The crate is less actively maintained than `svgo` itself.

Decision: stay with `usvg`. The README is honest about it: "canonicalize, not minify". The 13-point gap is acceptable for v1; revisit if real users complain.

### `--output-dir <DIR>` flag

Write optimized files into `<DIR>/<stem>_s<ext>` instead of overwriting the input. The flag is already declared in `cli.rs` (`Args::output_dir: Option<PathBuf>`) and parsed by clap, but the pipeline does not yet route through it. Wiring needs:

- Pass `output_dir` through `optimize_file` to the write site.
- Conflict policy: if `<stem>_s<ext>` exists in `<DIR>`, append `-1`, `-2`, ... rather than clobbering.
- `--no-backup` is implicit when `--output-dir` is set (the input is not touched).
- End-to-end test in `tests/output_dir.rs`.

### `--fail-fast` mode

Currently, processing continues after a per-file error and the exit code is 1 if any file failed. A `--fail-fast` flag would short-circuit on the first error. Useful for CI pipelines where any failure should stop the build.

### `*.tmp` cleanup

`write_atomic` writes to `path.tmp` and renames over the original. If the `rename` call fails, the `path.tmp` is left behind. The plan originally listed this as a "nice-to-have, not v1" — keeping it out of v1. A future `--clean-tmp` sweep or a startup scan for stale `*.tmp` files would be the fix.

### Release pipeline

No GitHub Actions / release automation yet. The plan stops at "single binary builds." Open questions for release:

- Pre-built binaries via `cargo dist` or `cross` for linux x86_64 / aarch64 / windows.
- `cargo install --path .` works locally but isn't a distribution story.
- Tagging convention (v0.1.0?) and CHANGELOG.
- Homebrew formula? `cargo install` is the documented install path.
