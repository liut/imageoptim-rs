use crate::optimize::Optimizer;
use anyhow::Context;

pub struct PngOptimizer;

impl Optimizer for PngOptimizer {
    fn optimize(
        &self,
        bytes: &[u8],
        _quality: Option<u8>,
        lossy: bool,
        no_zopfli: bool,
        max_colors: Option<u32>,
        png_level: Option<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        if lossy {
            optimize_lossy(bytes, no_zopfli, max_colors, png_level)
        } else {
            optimize_lossless(bytes, png_level)
        }
    }
}

fn optimize_lossless(bytes: &[u8], png_level: Option<u8>) -> anyhow::Result<Vec<u8>> {
    // Per-mode default: lossless PNG runs at preset 3 (balanced
    // speed/size). Users can override with `--png-optimization-level`.
    let level = png_level.unwrap_or(3);
    let opts = oxipng::Options::from_preset(level);
    oxipng::optimize_from_memory(bytes, &opts).map_err(|e| anyhow::anyhow!("oxipng: {e}"))
}

fn optimize_lossy(
    bytes: &[u8],
    no_zopfli: bool,
    max_colors: Option<u32>,
    png_level: Option<u8>,
) -> anyhow::Result<Vec<u8>> {
    // 1. Decode input to RGBA8 pixels.
    let (pixels, width, height) = decode_rgba(bytes)?;

    // 2. Quantize to a palette of up to `max_colors` colors (default
    //    imagequant cap of 256 when None).
    //    Mirrors ImageOptim.app's defaults: PngMinQuality=80 → quality
    //    range 80-100, level=4 → speed = MIN(3, 7-4) = 3.
    let mut attr = imagequant::Attributes::new();
    if let Some(n) = max_colors {
        attr.set_max_colors(n)
            .context("imagequant: set_max_colors")?;
    }
    attr.set_quality(80, 100)
        .context("imagequant: set_quality")?;
    attr.set_speed(3).context("imagequant: set_speed")?;
    let mut img = attr
        .new_image_borrowed(&pixels, width, height, 0.0)
        .context("imagequant: new_image")?;
    let mut res = attr.quantize(&mut img).context("imagequant: quantize")?;
    let (palette, indices) = res.remapped(&mut img).context("imagequant: remapped")?;

    // 3. Encode as an 8-bit palette PNG.
    let palette_png = encode_palette(width, height, &palette, &indices)?;

    // 4. Hand the palette PNG back through oxipng at max compression.
    //    The `zopfli` feature on oxipng is enabled in Cargo.toml, so
    //    Options::max_compression() (= from_preset(6)) routes the deflate
    //    stream through zopfli with --iterations=12. ImageOptim.app's
    //    ZopfliWorker runs --iterations=15 by default; we cap at 12 to
    //    keep wall-clock cost bounded. The user can override the
    //    preset with `--png-optimization-level`; per-mode default for
    //    the lossy inner step is 6 (max compression).
    let level = png_level.unwrap_or(6);
    let opts = oxipng::Options::from_preset(level);
    let mut current = oxipng::optimize_from_memory(&palette_png, &opts)
        .map_err(|e| anyhow::anyhow!("oxipng (post-pngquant): {e}"))?;

    // 5. Optional: hand off to `zopflipng` CLI for the final pass.
    //    zopflipng picks the best PNG filter combination and runs the
    //    deepest deflate search, which oxipng's preset 6 does not.
    //    Skipped silently if `--no-zopfli` is passed or the binary is
    //    not on $PATH.
    if !no_zopfli && let Some(zopfli_output) = run_zopflipng(&current) {
        current = zopfli_output;
    }

    Ok(current)
}

fn decode_rgba(bytes: &[u8]) -> anyhow::Result<(Vec<imagequant::RGBA>, usize, usize)> {
    let cursor = std::io::Cursor::new(bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().context("png decode")?;
    let info = reader.info();
    let w = info.width as usize;
    let h = info.height as usize;
    let color = info.color_type;
    let bit_depth = info.bit_depth as u8;

    // Decode into a contiguous RGBA8 buffer.
    let mut raw = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut raw).context("png next_frame")?;
    let buf = &raw[..frame.buffer_size()];

    let mut rgba: Vec<imagequant::RGBA> = Vec::with_capacity(w * h);
    match (color, bit_depth) {
        (png::ColorType::Rgba, 8) => {
            for px in buf.chunks_exact(4) {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[1],
                    b: px[2],
                    a: px[3],
                });
            }
        }
        (png::ColorType::Rgb, 8) => {
            for px in buf.chunks_exact(3) {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[1],
                    b: px[2],
                    a: 255,
                });
            }
        }
        (png::ColorType::GrayscaleAlpha, 8) => {
            for px in buf.chunks_exact(2) {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[0],
                    b: px[0],
                    a: px[1],
                });
            }
        }
        (png::ColorType::Grayscale, 8) => {
            for px in buf {
                rgba.push(imagequant::RGBA {
                    r: *px,
                    g: *px,
                    b: *px,
                    a: 255,
                });
            }
        }
        _ => {
            // For any other bit-depth / color-type combination, re-encode
            // via the image crate as a baseline 8-bit RGBA.
            let dyn_img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .context("png re-decode via image crate")?
                .to_rgba8();
            for px in dyn_img.pixels() {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[1],
                    b: px[2],
                    a: px[3],
                });
            }
        }
    }
    Ok((rgba, w, h))
}

fn encode_palette(
    width: usize,
    height: usize,
    palette: &[imagequant::RGBA],
    indices: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        // PLTE chunk
        let mut plte = Vec::with_capacity(palette.len() * 3);
        for c in palette {
            plte.push(c.r);
            plte.push(c.g);
            plte.push(c.b);
        }
        encoder.set_palette(plte);
        // tRNS chunk for partial alpha
        let trns: Vec<u8> = palette.iter().map(|c| c.a).collect();
        if trns.iter().any(|&a| a != 255) {
            encoder.set_trns(trns);
        }
        let mut writer = encoder.write_header().context("png write_header")?;
        writer
            .write_image_data(indices)
            .context("png write_image_data")?;
        writer.finish().context("png finish")?;
    }
    Ok(out)
}

/// Runs `zopflipng` as a subprocess on the bytes we already have, if it
/// is on `$PATH`. Returns `None` if the binary cannot be found (the
/// caller should fall through with the input unchanged, and prints a
/// one-time hint to stderr so the user knows they can install it).
///
/// Mirrors ImageOptim.app's `ZopfliWorker.m` defaults:
///   --filters=0pme, --iterations=15, --keepchunks=...
///
/// We pick `--filters=0pme` for moderate-size inputs (under 50 MB)
/// and `--filters=p` for large inputs, matching ImageOptim's
/// isLarge branch. We always strip ancillary chunks to match
/// `PngOutRemoveChunks=true`.
fn run_zopflipng(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static MISSING_WARNED: AtomicBool = AtomicBool::new(false);
    let zopflipng = match which("zopflipng") {
        Some(p) => p,
        None => {
            if !MISSING_WARNED.swap(true, Ordering::Relaxed) {
                eprintln!(
                    "imageoptim: optional tool `zopflipng` not found in $PATH; \
                     pass --no-zopfli to silence this hint, or install zopfli \
                     (macOS: brew install zopfli; Debian/Ubuntu: apt install zopfli) \
                     for the final ~10-20% savings on the --lossy PNG path."
                );
            }
            return None;
        }
    };

    let mut input = std::env::temp_dir();
    input.push(format!("imageoptim-zopfli-{}.png", std::process::id()));
    let mut output = input.clone();
    output.set_extension("out.png");

    // Use isLarge = (raw bytes > 50 MB). 50 MB matches ImageOptim's
    // heuristic. For our target use this branch is rarely hit.
    let is_large = bytes.len() > 50 * 1024 * 1024;
    let filters = if is_large { "p" } else { "0pme" };
    let iterations = 15;

    if std::fs::write(&input, bytes).is_err() {
        return None;
    }

    let result = std::process::Command::new(&zopflipng)
        .arg(format!("--filters={filters}"))
        .arg(format!("--iterations={iterations}"))
        .arg("--lossy_transparent")
        .arg("-y")
        .arg(&input)
        .arg(&output)
        .output();

    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(&output);

    let result = match result {
        Ok(r) if r.status.success() => r,
        Ok(r) => {
            eprintln!(
                "imageoptim: zopflipng exited with status {}; skipping the post-pass",
                r.status
            );
            return None;
        }
        Err(e) => {
            eprintln!("imageoptim: failed to invoke zopflipng: {e}");
            return None;
        }
    };
    let _ = result;
    std::fs::read(&output).ok()
}

/// Minimal `which(1)`: looks up `name` in `$PATH`. Returns the first
/// match. Returns `None` if not found.
fn which(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        // On Windows, executables carry an extension; check those too.
        #[cfg(windows)]
        {
            for ext in &["exe", "bat", "cmd"] {
                let with_ext = dir.join(format!("{name}.{ext}"));
                if with_ext.is_file() {
                    return Some(with_ext);
                }
            }
        }
    }
    None
}
