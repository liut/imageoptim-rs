use imageoptim::detect::Format;
use imageoptim::optimize::Optimizer;
use imageoptim::optimize::png::PngOptimizer;
use imageoptim::safety;

#[test]
fn detect_png_format() {
    let path = std::path::Path::new("foo.PNG");
    assert_eq!(Format::from_path(path), Some(Format::Png));
}

#[test]
fn detect_unknown_format() {
    let path = std::path::Path::new("foo.xyz");
    assert_eq!(Format::from_path(path), None);
}

#[test]
fn safety_rejects_empty_optimized() {
    let original = b"some bytes here";
    assert!(!safety::is_safe_to_write(original, b"", Format::Png));
}

#[test]
fn safety_rejects_larger_optimized() {
    let original = b"short";
    let larger = b"much longer than original";
    assert!(!safety::is_safe_to_write(original, larger, Format::Png));
}

#[test]
fn safety_rejects_equal_size() {
    let bytes = b"abc";
    assert!(!safety::is_safe_to_write(bytes, bytes, Format::Png));
}

#[test]
fn png_optimizer_produces_valid_output() {
    let optimizer = PngOptimizer;
    let png_bytes = make_png();
    let optimized = optimizer
        .optimize(&png_bytes, None)
        .expect("optimize should succeed");
    assert!(
        optimized.len() < png_bytes.len(),
        "optimized must be smaller"
    );
    assert!(
        safety::decode_valid(&optimized, Format::Png),
        "optimized PNG must be decodable"
    );
}

#[test]
fn png_optimizer_keeps_already_optimal() {
    let optimizer = PngOptimizer;
    let png_bytes = make_png();
    let optimized = optimizer.optimize(&png_bytes, None).expect("first pass");
    let optimized2 = optimizer.optimize(&optimized, None).expect("second pass");
    // After first optimization, the file should be at a local minimum.
    assert!(optimized2.len() <= optimized.len());
}

fn make_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(8, 8, |x, y| Rgb([(x * 32) as u8, (y * 32) as u8, 128]));
    let mut out = Vec::new();
    let dyn_img = image::DynamicImage::ImageRgb8(img);
    dyn_img
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}
