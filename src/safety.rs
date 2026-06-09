use crate::detect::Format;

/// Returns `true` when `optimized` is strictly smaller than `original` AND
/// successfully decodes back as `format`. This is the "safe write" contract.
pub fn is_safe_to_write(original: &[u8], optimized: &[u8], format: Format) -> bool {
    if optimized.is_empty() || optimized.len() >= original.len() {
        return false;
    }
    decode_valid(optimized, format)
}

pub fn decode_valid(bytes: &[u8], format: Format) -> bool {
    match format {
        Format::Png => png_valid(bytes),
        Format::Jpeg => jpeg_valid(bytes),
        Format::Gif => gif_valid(bytes),
        Format::Webp => webp_valid(bytes),
        Format::Svg => svg_valid(bytes),
    }
}

fn png_valid(bytes: &[u8]) -> bool {
    let cursor = std::io::Cursor::new(bytes);
    let Ok(mut decoder) = png::Decoder::new(cursor).read_info() else {
        return false;
    };
    let mut buf = vec![0u8; 1024];
    decoder.next_frame(&mut buf).is_ok()
}

fn jpeg_valid(bytes: &[u8]) -> bool {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.decode().is_ok()
}

fn gif_valid(bytes: &[u8]) -> bool {
    gif::Decoder::new(bytes).is_ok()
}

fn webp_valid(bytes: &[u8]) -> bool {
    webp::Decoder::new(bytes).decode().is_some()
}

fn svg_valid(bytes: &[u8]) -> bool {
    usvg::Tree::from_data(bytes, &usvg::Options::default()).is_ok()
}
