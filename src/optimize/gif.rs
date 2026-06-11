use crate::optimize::Optimizer;

pub struct GifOptimizer;

impl Optimizer for GifOptimizer {
    fn optimize(
        &self,
        bytes: &[u8],
        _quality: Option<u8>,
        _lossy: bool,
        _no_zopfli: bool,
        _max_colors: Option<u32>,
        _png_level: Option<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(bytes)?;

        let (w, h) = (decoder.width(), decoder.height());

        let mut frames: Vec<(u16, u16, Vec<u8>, u16)> = Vec::new();
        while let Some(frame) = decoder.read_next_frame()? {
            frames.push((
                frame.width,
                frame.height,
                frame.buffer.to_vec(),
                frame.delay,
            ));
        }
        if frames.is_empty() {
            return Ok(bytes.to_vec());
        }

        let mut out = Vec::with_capacity(bytes.len());
        {
            let mut encoder = gif::Encoder::new(&mut out, w, h, &[])?;
            for (fw, fh, rgba, delay) in frames {
                let mut f = gif::Frame::from_rgba(fw, fh, &mut rgba.clone());
                f.delay = delay;
                encoder.write_frame(&f)?;
            }
        }
        Ok(out)
    }
}
