use crate::detect::Format;

pub mod png;
pub mod jpeg;
pub mod gif;
pub mod webp;
pub mod svg;

/// Trait for format-specific optimizers.
pub trait Optimizer: Send + Sync {
    fn format(&self) -> Format;
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
