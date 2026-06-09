# imageoptim-rs

A cross-platform image optimization CLI powered by native Rust crates.

Inspired by [`JamieMason/ImageOptim-CLI`](https://github.com/JamieMason/ImageOptim-CLI) (3.5k stars, archived 2023-11), but unlike the original — which is a macOS-only AppleScript orchestrator that drives GUI applications — `imageoptim-rs` uses Rust crates directly. It runs on macOS, Linux, and Windows with no runtime dependencies beyond the C standard library, and ships as a single static binary.

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
| PNG | `oxipng` | Lossless (quality flag ignored) |
| JPEG | `jpeg-decoder` + `jpeg-encoder` | Lossy re-encoding, default quality 85 |
| GIF | `gif` crate | Indexed re-encoding with NeuQuant (quality flag ignored) |
| WebP | `webp` + `image` | Lossy re-encoding when `--quality` is set, lossless otherwise |
| SVG | `usvg` | Canonical re-serialization; not a full minifier (quality flag ignored) |

`--quality <0-100>` controls the lossy quality for JPEG and WebP. Lower values produce smaller files at the cost of visual fidelity. It is silently ignored for PNG, GIF, and SVG, which are always lossless.

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
  -q, --quality <0-100>  Quality for lossy formats (0-100). Omit for lossless
  -j, --jobs <N>         Number of parallel workers
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

### Backups (on by default)

Before overwriting, `imageoptim-rs` copies the original file to `<path>.bak`. The backup is created on the **first** run for each file and is never overwritten by subsequent runs. To restore from backup:

```bash
mv foo.png.bak foo.png
```

Backups are skipped in `--dry-run` mode and can be disabled entirely with `--no-backup` (the file is still optimized in place, just without the `.bak` copy). They live next to the originals, so the file count roughly doubles during the first optimization pass — remember to clean them up once you're satisfied.

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

MIT — see `LICENSE`.

## Acknowledgments

- [`JamieMason/ImageOptim-CLI`](https://github.com/JamieMason/ImageOptim-CLI) — the original concept and CLI surface
- All the Rust crate authors whose libraries make this possible: `oxipng`, `gif`, `webp`, `usvg`, `image`, `jpeg-decoder`, `jpeg-encoder`
