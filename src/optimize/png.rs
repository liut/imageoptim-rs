use crate::optimize::Optimizer;
use anyhow::Context;

pub struct PngOptimizer;

impl Optimizer for PngOptimizer {
    fn optimize(&self, bytes: &[u8], _quality: Option<u8>, lossy: bool) -> anyhow::Result<Vec<u8>> {
        if lossy {
            optimize_lossy(bytes)
        } else {
            optimize_lossless(bytes)
        }
    }
}

fn optimize_lossless(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let opts = oxipng::Options::from_preset(3);
    oxipng::optimize_from_memory(bytes, &opts).map_err(|e| anyhow::anyhow!("oxipng: {e}"))
}

fn optimize_lossy(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    // 1. Decode input to RGBA8 pixels.
    let (pixels, width, height) = decode_rgba(bytes)?;

    // 2. Quantize to a palette of at most 256 colors.
    //    Mirrors ImageOptim.app's defaults: PngMinQuality=80 → quality
    //    range 80-100, level=4 → speed = MIN(3, 7-4) = 3.
    let mut attr = imagequant::Attributes::new();
    attr.set_quality(80, 100)
        .context("imagequant: set_quality")?;
    attr.set_speed(3).context("imagequant: set_speed")?;
    let mut img = attr
        .new_image_borrowed(&pixels, width, height, 0.0)
        .context("imagequant: new_image")?;
    let mut res = attr.quantize(&mut img).context("imagequant: quantize")?;
    let (palette, indices) = res.remapped(&mut img).context("imagequant: remapped")?;

    // 3. Encode as an 8-bit palette PNG.
    let palette_png = encode_palette(width, height, &palette, &indices)?;

    // 4. Hand the palette PNG back through oxipng (preset 4, matching
    //    ImageOptim's default AdvPngLevel=4). The palette is small but
    //    the indexed pixel data still has zlib entropy that oxipng can
    //    crush further with the right filter / window combo. This is
    //    the step that takes us from ~32% to ~64% on photographic PNGs.
    let opts = oxipng::Options::from_preset(4);
    oxipng::optimize_from_memory(&palette_png, &opts)
        .map_err(|e| anyhow::anyhow!("oxipng (post-pngquant): {e}"))
}

fn decode_rgba(bytes: &[u8]) -> anyhow::Result<(Vec<imagequant::RGBA>, usize, usize)> {
    let cursor = std::io::Cursor::new(bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().context("png decode")?;
    let info = reader.info();
    let w = info.width as usize;
    let h = info.height as usize;
    let color = info.color_type;
    let bit_depth = info.bit_depth as u8;

    // Decode into a contiguous RGBA8 buffer.
    let mut raw = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut raw).context("png next_frame")?;
    let buf = &raw[..frame.buffer_size()];

    let mut rgba: Vec<imagequant::RGBA> = Vec::with_capacity(w * h);
    match (color, bit_depth) {
        (png::ColorType::Rgba, 8) => {
            for px in buf.chunks_exact(4) {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[1],
                    b: px[2],
                    a: px[3],
                });
            }
        }
        (png::ColorType::Rgb, 8) => {
            for px in buf.chunks_exact(3) {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[1],
                    b: px[2],
                    a: 255,
                });
            }
        }
        (png::ColorType::GrayscaleAlpha, 8) => {
            for px in buf.chunks_exact(2) {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[0],
                    b: px[0],
                    a: px[1],
                });
            }
        }
        (png::ColorType::Grayscale, 8) => {
            for px in buf {
                rgba.push(imagequant::RGBA {
                    r: *px,
                    g: *px,
                    b: *px,
                    a: 255,
                });
            }
        }
        _ => {
            // For any other bit-depth / color-type combination, re-encode
            // via the image crate as a baseline 8-bit RGBA.
            let dyn_img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .context("png re-decode via image crate")?
                .to_rgba8();
            for px in dyn_img.pixels() {
                rgba.push(imagequant::RGBA {
                    r: px[0],
                    g: px[1],
                    b: px[2],
                    a: px[3],
                });
            }
        }
    }
    Ok((rgba, w, h))
}

fn encode_palette(
    width: usize,
    height: usize,
    palette: &[imagequant::RGBA],
    indices: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        // PLTE chunk
        let mut plte = Vec::with_capacity(palette.len() * 3);
        for c in palette {
            plte.push(c.r);
            plte.push(c.g);
            plte.push(c.b);
        }
        encoder.set_palette(plte);
        // tRNS chunk for partial alpha
        let trns: Vec<u8> = palette.iter().map(|c| c.a).collect();
        if trns.iter().any(|&a| a != 255) {
            encoder.set_trns(trns);
        }
        let mut writer = encoder.write_header().context("png write_header")?;
        writer
            .write_image_data(indices)
            .context("png write_image_data")?;
        writer.finish().context("png finish")?;
    }
    Ok(out)
}
