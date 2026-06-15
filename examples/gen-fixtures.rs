//! Generate a synthetic 1122x1402 RGB photo fixture used by the test
//! suite. The output is committed to `.gitignore`, not to git history.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example gen-fixtures
//! ```
//!
//! Pass `--force` to overwrite an existing fixture:
//!
//! ```sh
//! cargo run --example gen-fixtures -- --force
//! ```
//!
//! Without `--force`, the script refuses to overwrite an existing
//! `tests/example01.png`. The fixture is 2.3 MB and the test suite
//! assumes the realistic (unoptimized) bytes; if a previous run of
//! `imageoptim` accidentally shrunk the fixture in place, the
//! integration tests that compare output sizes against the original
//! start reporting "skipped" instead of "saved" and silently
//! regression. The `--force` flag makes the overwrite intentional.
//!
//! Why a script and not a checked-in file: a 2.3 MB binary file is
//! noise in the repository and in `git clone` size. Tests only need a
//! decodable, larger-than-its-optimized-output PNG. The image itself is
//! procedural — a smooth color gradient with seeded noise so that the
//! default and `--lossy` paths are forced to do real work rather than
//! collapsing on a flat color.

use image::{ImageBuffer, Rgb, RgbImage};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let force = args.iter().any(|a| a == "--force");

    let out: PathBuf = ["tests", "example01.png"].iter().collect();
    if out.exists() && !force {
        eprintln!(
            "{} already exists ({} bytes). Pass --force to overwrite.",
            out.display(),
            std::fs::metadata(&out).expect("stat").len()
        );
        std::process::exit(1);
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("create tests/ dir");
    }

    let width: u32 = 1122;
    let height: u32 = 1402;
    let mut img: RgbImage = ImageBuffer::new(width, height);

    // Simple LCG so the noise is deterministic across runs and platforms.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };

    for y in 0..height {
        for x in 0..width {
            // Smooth vertical gradient (sky-to-ground) — broad color
            // regions, the kind of thing that real photos have and
            // that palette quantization can compress well.
            let t = y as f32 / height as f32;
            let base_r = (40.0 + 180.0 * t) as u8;
            let base_g = (90.0 + 110.0 * (1.0 - t)) as u8;
            let base_b = (200.0 - 140.0 * t) as u8;

            // Diagonal hue band.
            let band = ((x as f32 / width as f32) * 4.0).sin().abs();
            let band_byte = (band * 50.0) as u8;

            // Per-pixel deterministic noise (amplitude 6). Generates
            // ~2.9 MB on disk. imagequant at the 80-100 quality target
            // will reject very small palette caps (N=2, N=16) on this
            // image because the noise prevents hitting the quality
            // threshold. That behavior is captured by the
            // `max_colors_clamps_at_two` and `max_colors_reduces_*`
            // tests, which assert either saved-with-reduction OR
            // no-change-with-error.
            let n = (next() % 12) as i32 - 6;

            let r = (base_r as i32 + band_byte as i32 + n).clamp(0, 255) as u8;
            let g = (base_g as i32 + n).clamp(0, 255) as u8;
            let b = (base_b as i32 - band_byte as i32 + n).clamp(0, 255) as u8;

            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }

    img.save(&out).expect("write tests/example01.png");
    let size = std::fs::metadata(&out).expect("stat").len();
    eprintln!("wrote {} ({} bytes)", out.display(), size);
}
