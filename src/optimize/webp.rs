use crate::optimize::Optimizer;

pub struct WebpOptimizer;

impl Optimizer for WebpOptimizer {
    fn optimize(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let decoder = webp::Decoder::new(bytes);
        let image = decoder
            .decode()
            .ok_or_else(|| anyhow::anyhow!("WebP decode failed"))?;
        let dynamic = image.to_image();
        let encoder = webp::Encoder::from_image(&dynamic)
            .map_err(|e| anyhow::anyhow!("webp encoder: {e}"))?;
        let memory = encoder.encode_lossless();
        Ok(memory.to_vec())
    }
}
