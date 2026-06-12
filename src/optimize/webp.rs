use crate::optimize::{Optimizer, OptimizerOptions};

pub struct WebpOptimizer;

impl Optimizer for WebpOptimizer {
    fn optimize(&self, bytes: &[u8], opts: &OptimizerOptions) -> anyhow::Result<Vec<u8>> {
        let decoder = webp::Decoder::new(bytes);
        let image = decoder
            .decode()
            .ok_or_else(|| anyhow::anyhow!("WebP decode failed"))?;
        let dynamic = image.to_image();
        let encoder = webp::Encoder::from_image(&dynamic)
            .map_err(|e| anyhow::anyhow!("webp encoder: {e}"))?;
        let memory = match opts.quality {
            Some(q) => encoder.encode(q as f32),
            None => encoder.encode_lossless(),
        };
        Ok(memory.to_vec())
    }
}
