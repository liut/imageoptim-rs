use crate::detect::Format;
use crate::optimize::Optimizer;

pub struct GifOptimizer;

impl Optimizer for GifOptimizer {
    fn format(&self) -> Format {
        Format::Gif
    }

    fn optimize(&self, bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::Indexed);
        let mut decoder = options.read_info(bytes)?;

        let (w, h) = (decoder.width(), decoder.height());
        let palette = decoder.global_palette().unwrap_or(&[]).to_vec();

        let mut frames: Vec<Vec<u8>> = Vec::new();
        let mut delays: Vec<u16> = Vec::new();
        while let Some(frame) = decoder.read_next_frame()? {
            frames.push(frame.buffer.to_vec());
            delays.push(frame.delay);
        }
        if frames.is_empty() {
            return Ok(bytes.to_vec());
        }

        let mut out = Vec::with_capacity(bytes.len());
        {
            let mut encoder = if palette.is_empty() {
                gif::Encoder::new(&mut out, w, h, &[])
            } else {
                gif::Encoder::new(&mut out, w, h, &palette)
            }?;
            for (rgba, delay) in frames.iter().zip(delays.iter()) {
                let mut frame = gif::Frame::from_rgba(w, h, &mut rgba.clone());
                frame.delay = *delay;
                encoder.write_frame(&frame)?;
            }
        }
        Ok(out)
    }
}
