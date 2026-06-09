use imageoptim::detect::Format;
use imageoptim::optimize::Optimizer;
use imageoptim::optimize::gif::GifOptimizer;
use imageoptim::optimize::jpeg::JpegOptimizer;
use imageoptim::optimize::svg::SvgOptimizer;
use imageoptim::optimize::webp::WebpOptimizer;
use imageoptim::safety;

fn make_jpeg() -> Vec<u8> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 32) as u8, (y * 32) as u8, 128]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut out),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    out
}

fn make_gif() -> Vec<u8> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 32) as u8, (y * 32) as u8, 64]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Gif)
        .unwrap();
    out
}

fn make_webp() -> Vec<u8> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 32) as u8, (y * 32) as u8, 64]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut out),
            image::ImageFormat::WebP,
        )
        .unwrap();
    out
}

fn make_svg() -> Vec<u8> {
    br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect x="0" y="0" width="10" height="10" fill="red"/></svg>"#.to_vec()
}

#[test]
fn jpeg_round_trip() {
    let optimizer = JpegOptimizer;
    let input = make_jpeg();
    let output = optimizer
        .optimize(&input, None, false)
        .expect("jpeg optimize");
    if output.len() < input.len() {
        assert!(
            safety::decode_valid(&output, Format::Jpeg),
            "JPEG output invalid"
        );
    }
}

#[test]
fn gif_round_trip() {
    let optimizer = GifOptimizer;
    let input = make_gif();
    let output = optimizer
        .optimize(&input, None, false)
        .expect("gif optimize");
    if output.len() < input.len() {
        assert!(
            safety::decode_valid(&output, Format::Gif),
            "GIF output invalid"
        );
    }
}

#[test]
fn webp_round_trip() {
    let optimizer = WebpOptimizer;
    let input = make_webp();
    let output = optimizer
        .optimize(&input, None, false)
        .expect("webp optimize");
    if output.len() < input.len() {
        assert!(
            safety::decode_valid(&output, Format::Webp),
            "WebP output invalid"
        );
    }
}

#[test]
fn svg_round_trip() {
    let optimizer = SvgOptimizer;
    let input = make_svg();
    let output = optimizer
        .optimize(&input, None, false)
        .expect("svg optimize");
    if output.len() < input.len() {
        assert!(
            safety::decode_valid(&output, Format::Svg),
            "SVG output invalid"
        );
    }
}

#[test]
fn jpeg_quality_affects_output_size() {
    let optimizer = JpegOptimizer;
    let input = make_jpeg();
    let high = optimizer.optimize(&input, Some(95), false).expect("q=95");
    let low = optimizer.optimize(&input, Some(20), false).expect("q=20");
    assert!(
        low.len() < high.len(),
        "q=20 ({}) should produce smaller output than q=95 ({})",
        low.len(),
        high.len()
    );
}

#[test]
fn png_lossy_smaller_than_lossless() {
    use imageoptim::optimize::png::PngOptimizer;
    // Use the real-world fixture committed in tests/example01.png — a
    // 2.3 MB RGB photo PNG. For natural images, the lossy palette
    // path is expected to beat the lossless zlib-only path.
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example01.png");
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }
    let input = std::fs::read(&fixture).expect("read fixture");
    let optimizer = PngOptimizer;
    let lossless = optimizer.optimize(&input, None, false).expect("lossless");
    let lossy = optimizer.optimize(&input, None, true).expect("lossy");
    assert!(
        lossy.len() < lossless.len(),
        "lossy ({}) must be smaller than lossless ({}) for a real photo",
        lossy.len(),
        lossless.len()
    );
    // Lossy output must still be a valid decodable PNG.
    assert!(
        safety::decode_valid(&lossy, Format::Png),
        "lossy PNG output must decode successfully"
    );
}
