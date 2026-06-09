use crate::optimize::Optimizer;

const DEFAULT_QUALITY: u8 = 85;

pub struct JpegOptimizer;

impl Optimizer for JpegOptimizer {
    fn optimize(&self, bytes: &[u8], quality: Option<u8>, _lossy: bool) -> anyhow::Result<Vec<u8>> {
        let q = quality.unwrap_or(DEFAULT_QUALITY);
        let mut decoder = jpeg_decoder::Decoder::new(bytes);
        let pixels = decoder
            .decode()
            .map_err(|e| anyhow::anyhow!("jpeg-decoder: {e}"))?;
        let info = decoder
            .info()
            .ok_or_else(|| anyhow::anyhow!("missing JPEG info"))?;
        let color = map_color(info.pixel_format);
        let mut out = Vec::with_capacity(bytes.len());
        let encoder = jpeg_encoder::Encoder::new(&mut out, q);
        encoder
            .encode(&pixels, info.width, info.height, color)
            .map_err(|e| anyhow::anyhow!("jpeg-encoder: {e}"))?;
        Ok(out)
    }
}

fn map_color(p: jpeg_decoder::PixelFormat) -> jpeg_encoder::ColorType {
    match p {
        jpeg_decoder::PixelFormat::L8 => jpeg_encoder::ColorType::Luma,
        jpeg_decoder::PixelFormat::RGB24 => jpeg_encoder::ColorType::Rgb,
        jpeg_decoder::PixelFormat::CMYK32 => jpeg_encoder::ColorType::Cmyk,
        _ => jpeg_encoder::ColorType::Rgb,
    }
}
