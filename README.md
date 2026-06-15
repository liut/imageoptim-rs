# imageoptim-rs

A cross-platform image optimization CLI powered by native Rust crates.

Inspired by [`JamieMason/ImageOptim-CLI`](https://github.com/JamieMason/ImageOptim-CLI) (3.5k stars, archived 2023-11), but unlike the original — which is a macOS-only AppleScript orchestrator that drives GUI applications — `imageoptim-rs` uses Rust crates directly. It runs on macOS, Linux, and Windows with no runtime dependencies beyond the C standard library, and ships as a single static binary.

> **License notice.** `imageoptim-rs` is licensed under **GPL-3.0-or-later**. This is a copyleft license: any binary you distribute that links against this code must also be GPL-3.0-or-later, and you must provide source. The GPL license is required because the optional lossy PNG path links against [libimagequant](https://github.com/ImageOptim/libimagequant), which is GPL. If you cannot accept GPL terms, do not use the binary.

## Install

```bash
cargo install --git https://github.com/yourname/imageoptim-rs
```

Or build from source:

```bash
git clone https://github.com/yourname/imageoptim-rs
cd imageoptim-rs
cargo build --release
./target/release/imageoptim --help
```

## Quickstart

Optimize every PNG in the current directory, in place:

```bash
imageoptim '*.png'
```

Recurse into subdirectories and process only JPEGs:

```bash
imageoptim '**/*.jpg' -r
```

Preview what would happen without modifying any files:

```bash
imageoptim 'assets/**/*.png' -r --dry-run
```

Use 4 parallel workers:

```bash
imageoptim '**/*.{png,jpg,gif,webp,svg}' -j 4
```

## Supported formats

| Format | Optimizer | Notes |
| --- | --- | --- |
| PNG | `oxipng` (lossless) or `imagequant` (lossy with `--lossy`) | Lossless by default; `--lossy` quantizes to up to 256 colors, tunable via `--max-colors`; oxipng preset is tunable via `--png-optimization-level` |
| JPEG | `jpeg-decoder` + `jpeg-encoder` | Lossy re-encoding, default quality 85; EXIF/IPTC/XMP/ICC/comment markers in the input are dropped implicitly (the encoder re-encodes from raw pixels and only emits the required JFIF APP0 header) |
| GIF | `gif` crate | Indexed re-encoding with NeuQuant (quality flag ignored) |
| WebP | `webp` + `image` | Lossy re-encoding when `--quality` is set, lossless otherwise |
| SVG | `usvg` | Canonical re-serialization; not a full minifier (quality flag ignored) |

`--quality <0-100>` controls the lossy quality for JPEG and WebP. It is silently ignored for GIF and SVG, which are always lossless.

`--lossy` enables palette quantization for PNG. This is the same algorithm that powers [pngquant](https://pngquant.org/) and the "Lossy" checkbox in ImageOptim.app: each pixel's color is mapped to the nearest entry in a palette of up to 256 colors, which can shrink photographic PNGs by 50–80% at the cost of subtle color banding. The output remains a valid PNG, but it is no longer byte-identical to the original — keep the `.bak` (or pass `--dry-run`) when you first try it on real assets. Use `--max-colors <N>` (2..=256, requires `--lossy`) to cap the palette size; smaller values mean more banding but a smaller file.

## Flags

```
Usage: imageoptim [OPTIONS] [PATTERN]...

Arguments:
  [PATTERN]...  File paths or glob patterns (e.g. `*.png`, `assets/**/*.jpg`)

Options:
  -r, --recursive        Recurse into directories
      --dry-run          Show what would be done without modifying any files
      --no-color         Disable ANSI color output
      --no-backup        Skip creating `<path>.bak` before overwriting
      --lossy            Allow lossy PNG palette quantization (off by default)
      --max-colors <N>   Cap the lossy palette at N colors (2-256, requires --lossy)
      --no-zopfli        Skip the optional `zopflipng` post-pass on `--lossy`
      --png-optimization-level <0-6>  Override the oxipng preset (default 3 lossless, 6 lossy)
      --output-dir <DIR> Write optimized files into `<DIR>/<stem>_s<ext>` instead of overwriting
      --fail-fast        Stop processing on the first per-file error
  -q, --quality <0-100>  Quality for lossy formats (0-100). Omit for lossless
  -j, --jobs <N>         Number of parallel workers
  -v, --verbose          Print per-step optimization details to stderr
      --summary-only     Suppress per-file result lines; print only the summary
  -h, --help             Print help
  -V, --version          Print version
```

## Safety contract

`imageoptim-rs` will only overwrite a file when **both** of the following are true:

1. The optimized output is strictly smaller than the original.
2. The optimized output decodes back to a valid image of the same format.

If either condition fails — for example, a file is already optimally compressed, or the encoder produced a malformed result — the file is left untouched and reported as `skipped`.

A file that fails to optimize (e.g. oxipng errors on a corrupt PNG) does not overwrite the original; processing continues for sibling files. The process exits with status code 1 if any file failed.

### Progress bar

When stderr is attached to a terminal, `imageoptim-rs` draws a progress bar during processing. The bar is automatically suppressed when:

- stdout/stderr is redirected (e.g. piped to a file or another command), so logs stay clean
- `--dry-run` is set, since there is nothing to wait on

### Optional `zopflipng` post-pass

The `--lossy` PNG pipeline runs three steps:

1. **pngquant** quantizes the image to a 256-color palette (using libimagequant, embedded).
2. **oxipng** at max compression re-encodes the palette PNG, with `--iterations=12` of zopfli-deflate search built into oxipng.
3. **`zopflipng`** (if installed) re-runs the PNG filter selection and the deeper deflate search — typically another 10–20% savings on top.

Step 3 is invoked automatically when `zopflipng` is found in `$PATH`. The first run on a system without it prints one hint to stderr pointing to the install command. Pass `--no-zopfli` to skip the step (and silence the hint) entirely.

Install options:

- macOS: `brew install zopfli`
- Debian / Ubuntu: `apt install zopfli`
- From source: <https://github.com/google/zopfli>

### Verbose mode (`-v` / `--verbose`)

Pass `-v` to print per-step optimization details to stderr. For a PNG run, the trace looks like:

```
imageoptim: png lossy → decoded 1122x1402 RGBA8 (1573044 pixels)
imageoptim:   imagequant q=80-100 max_colors=256 speed=3
imageoptim:   imagequant produced 256 entries in the palette
imageoptim:   oxipng preset 6 (zopfli iterations=12)
imageoptim:   zopflipng not installed; skipped
  [PNG] tests/example01.png saved 1.42 MB (64.04%)
```

The trace distinguishes "zopflipng not installed" from "zopflipng installed but failed" so you can tell at a glance whether installing `zopfli` would help. Other formats (JPEG, GIF, WebP, SVG) don't emit trace lines — their per-file result line carries the only meaningful detail. The per-file result line and the summary are unchanged; the trace is purely additive.

### `--summary-only`

Pass `--summary-only` to suppress the per-file `saved/skipped` line from stdout. The aggregate summary is still printed, and any failures still go to stderr. Useful in CI where you only care about the aggregate delta:

```
$ imageoptim --summary-only assets/**/*.png
Processed 47 files, saved 12.34 MB (38.21%)
```

Numbers on `tests/example01.png` (a 2.89 MB synthetic 1122×1402 RGB photo generated by `cargo run --example gen-fixtures`; the file is git-ignored — see "Test fixtures" under [Development](#development)):

| Path | Output | Savings |
| --- | --- | --- |
| `--png-optimization-level 0` (fastest) | 799 KB | 72.34% |
| Default (`oxipng` preset 3) | 1.97 MB | 14.86% (inferred from preset 0; the synthetic image is dominated by noise) |
| `--lossy` (pngquant + oxipng max + zopfli-in-oxipng) | 560 KB | 80.61% |
| `--lossy --max-colors 128` | 478 KB | 83.45% |
| `--lossy --max-colors 16` | rejected | imagequant cannot meet the 80-100 quality target with 16 colors on this noisy synthetic image; real photos compress further (286 KB / 87.67% on a real photo previously measured) |
| `--lossy` with `zopflipng` installed (estimated) | ~250 KB | ~91% |

### Backups (on by default)

Before overwriting, `imageoptim-rs` copies the original file to `<path>.bak`. The backup is created on the **first** run for each file and is never overwritten by subsequent runs. To restore from backup:

```bash
mv foo.png.bak foo.png
```

Backups are skipped in `--dry-run` mode and can be disabled entirely with `--no-backup` (the file is still optimized in place, just without the `.bak` copy). They live next to the originals, so the file count roughly doubles during the first optimization pass — remember to clean them up once you're satisfied.

### Output directory (no in-place writes)

`--output-dir <DIR>` writes each optimized file to `<DIR>/<stem>_s<ext>` instead of overwriting the input in place. The input is left untouched, so `--no-backup` is implicit and no `.bak` files are produced. The target directory is created if it does not exist.

If `<stem>_s<ext>` already exists in `<DIR>`, a numeric suffix is appended: `foo_s-1.png`, `foo_s-2.png`, etc. Nothing is ever clobbered silently.

```bash
# Side-by-side comparison: original next to optimized
imageoptim assets/*.png --output-dir out/

# assets/foo.png  →  out/foo_s.png
# assets/bar.jpg  →  out/bar_s.jpg
```

This is the recommended way to A/B-compare results before committing to a rewrite in place.

### Fail-fast

By default, every file is processed even after a per-file error, and the final exit code is 1 if any file failed. Pass `--fail-fast` to short-circuit and exit immediately on the first error. Useful in CI pipelines where any failure should stop the build.

## Development

### Test fixtures

The integration tests read a single 2-3 MB photo fixture at
`tests/example01.png`. To keep the repository lean, this file is
git-ignored and generated on demand by a small example program:

```sh
cargo run --example gen-fixtures
```

This writes a 1122×1402 RGB PNG that is shaped like a natural photo
(smooth color regions with low-amplitude noise) so the lossy palette
quantizer has real work to do. The output is deterministic — a seeded
LCG — so the bytes match across runs and platforms.

Without this step, fixture-dependent tests skip silently (they
print `skipping: tests/example01.png not present` and return 0). All
other tests run as normal. To run the full suite end-to-end:

```sh
cargo run --example gen-fixtures && cargo test
```

If `tests/example01.png` already exists, `gen-fixtures` refuses to
overwrite (it's a destructive op on a 2.89 MB fixture that you may have
regenerated deliberately). Pass `--force` to overwrite:

```sh
cargo run --example gen-fixtures -- --force
```

### Cross-platform builds

The code is portable pure Rust: no `#[cfg(target_os = ...)]`
mismatches, no `sh -c` shell-outs, path handling via `std::path::Path`
consistently. The only platform-specific branch is in `which()`'s
Windows extension probe (`#[cfg(windows)]` for `.exe`/`.bat`/`.cmd`).

CI on GitHub Actions builds and tests on
`ubuntu-latest` / `macos-latest` / `windows-latest` on every push and
PR to `main` (see `.github/workflows/ci.yml`). The release workflow
(`.github/workflows/release.yml`) cross-compiles prebuilt binaries for
`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`aarch64-apple-darwin`, and `x86_64-pc-windows-msvc` on every
`v*.*.*` tag push, attaches them to a GitHub Release.

For local cross-compile (no CI):

- Install the std library for the target: `rustup target add
  <triple>`. Requires network access to `static.rust-lang.org`.
- Then `cargo check --target <triple>` verifies the crate's own
  code compiles for that target. The first build of an unfamiliar
  target downloads the sysroot (~100 MB) so the initial run can
  take 1-2 minutes.
- For a real binary, you also need the platform C toolchain
  (`mingw-w64` for Windows GNU, MSVC build tools for Windows MSVC,
  `musl-tools` for fully static Linux binaries).

## Comparison to ImageOptim-CLI

| | ImageOptim-CLI | imageoptim-rs |
| --- | --- | --- |
| Platforms | macOS only | macOS, Linux, Windows |
| Runtime deps | Three macOS GUI apps | None (single static binary) |
| Implementation | TypeScript + AppleScript | Pure Rust |
| Maintenance | Archived 2023-11 | Active |
| Format auto-detect | Yes | Yes |
| Glob support | Yes | Yes |
| Recursive | Yes | Yes |
| Dry-run | No | Yes |
| Cross-format conversion (PNG→WebP) | No | No (out of scope) |

## License

GPL-3.0-or-later — see `LICENSE`. Binaries linking this code (including the default build with the lossy PNG path) must also be distributed under GPL-3.0-or-later, and you must provide source to your recipients.

## Acknowledgments

- [`JamieMason/ImageOptim-CLI`](https://github.com/JamieMason/ImageOptim-CLI) — the original concept and CLI surface
- All the Rust crate authors whose libraries make this possible: `oxipng`, `gif`, `webp`, `usvg`, `image`, `jpeg-decoder`, `jpeg-encoder`
