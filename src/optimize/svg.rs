use crate::optimize::Optimizer;

pub struct SvgOptimizer;

impl Optimizer for SvgOptimizer {
    fn optimize(
        &self,
        bytes: &[u8],
        _quality: Option<u8>,
        _lossy: bool,
        _no_zopfli: bool,
        _max_colors: Option<u32>,
        _png_level: Option<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let tree = usvg::Tree::from_data(bytes, &usvg::Options::default())?;
        let xml = tree.to_string(&usvg::WriteOptions::default());
        Ok(xml.into_bytes())
    }
}
