use crate::optimize::Optimizer;

pub struct PngOptimizer;

impl Optimizer for PngOptimizer {
    fn optimize(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let opts = oxipng::Options::from_preset(3);
        oxipng::optimize_from_memory(bytes, &opts).map_err(|e| anyhow::anyhow!("oxipng: {e}"))
    }
}
