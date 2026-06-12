use crate::detect::Format;

pub mod gif;
pub mod jpeg;
pub mod png;
pub mod svg;
pub mod webp;

/// Per-call options consumed by [`Optimizer::optimize`].
///
/// Most fields are honored by a subset of formats only:
/// - `quality`: 0-100 lossy quality hint. Honored by JPEG and WebP;
///   ignored by GIF, SVG, and the lossless PNG path.
/// - `lossy`: enables palette quantization on the PNG path. Honored
///   by PNG; ignored by other formats (which already pick their own
///   lossless/lossy behavior).
/// - `no_zopfli`: suppresses the optional `zopflipng` CLI post-pass
///   on the lossy PNG path. Ignored for other formats.
/// - `max_colors`: caps the palette size used by the lossy PNG path
///   (clamped to imagequant's `2..=256` range at the CLI). Ignored
///   for other formats.
/// - `png_level`: overrides the oxipng preset (0..=6) used for the
///   PNG inner step. Higher is slower + smaller. Ignored for
///   non-PNG formats. The `None` default preserves the per-mode
///   defaults baked into the PNG optimizer (3 for lossless, 6 for
///   the lossy inner step).
/// - `verbose`: when true, optimizers may emit per-step progress
///   information to stderr (which oxipng preset was used, whether
///   the optional `zopflipng` CLI ran, etc.). Has no effect on the
///   output bytes; only on the diagnostic stream.
///
/// The struct replaces what used to be five positional args on
/// `Optimizer::optimize`. New flags add fields here instead of
/// widening the trait signature.
#[derive(Debug, Clone, Default)]
pub struct OptimizerOptions {
    pub quality: Option<u8>,
    pub lossy: bool,
    pub no_zopfli: bool,
    pub max_colors: Option<u32>,
    pub png_level: Option<u8>,
    pub verbose: bool,
}

/// Trait for format-specific optimizers.
pub trait Optimizer: Send + Sync {
    fn optimize(&self, bytes: &[u8], opts: &OptimizerOptions) -> anyhow::Result<Vec<u8>>;
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
