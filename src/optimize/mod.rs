use crate::detect::Format;

pub mod gif;
pub mod jpeg;
pub mod png;
pub mod svg;
pub mod webp;

/// Trait for format-specific optimizers.
///
/// `quality` is a 0-100 lossy quality hint. It is honored by lossy
/// formats (JPEG, WebP) and ignored by lossless formats (GIF, SVG).
/// For PNG, `lossy=true` enables palette quantization via libimagequant
/// (reduces the image to up to 256 colors); when `lossy=false` the PNG
/// is recompressed losslessly with `oxipng`.
/// `no_zopfli` suppresses the optional `zopflipng` CLI post-pass on
/// the lossy PNG path; ignored for other formats.
pub trait Optimizer: Send + Sync {
    fn optimize(
        &self,
        bytes: &[u8],
        quality: Option<u8>,
        lossy: bool,
        no_zopfli: bool,
    ) -> anyhow::Result<Vec<u8>>;
}

pub fn for_format(format: Format) -> Box<dyn Optimizer> {
    match format {
        Format::Png => Box::new(png::PngOptimizer),
        Format::Jpeg => Box::new(jpeg::JpegOptimizer),
        Format::Gif => Box::new(gif::GifOptimizer),
        Format::Webp => Box::new(webp::WebpOptimizer),
        Format::Svg => Box::new(svg::SvgOptimizer),
    }
}
