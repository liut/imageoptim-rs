use crate::detect::Format;

pub mod gif;
pub mod jpeg;
pub mod png;
pub mod svg;
pub mod webp;

/// Trait for format-specific optimizers.
pub trait Optimizer: Send + Sync {
    fn optimize(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>>;
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
