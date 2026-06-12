use imageoptim::optimize::Optimizer;
use imageoptim::optimize::OptimizerOptions;
use imageoptim::optimize::png::PngOptimizer;

#[test]
fn png_optimizer_no_zopfli_flag_honored() {
    // When `no_zopfli=true`, the optimizer must still produce a valid,
    // smaller-than-lossless PNG. The behavior with or without zopflipng
    // available on the host should not differ in this assertion
    // (zopflipng only makes the output smaller, never larger).
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example01.png");
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }
    let bytes = std::fs::read(&fixture).unwrap();
    let optimizer = PngOptimizer;
    let opts = OptimizerOptions {
        lossy: true,
        no_zopfli: true,
        ..Default::default()
    };
    let out = optimizer
        .optimize(&bytes, &opts)
        .expect("lossy + no_zopfli");
    assert!(out.len() < bytes.len(), "must shrink the input");
    assert!(
        out[0] == 0x89 && out[1] == b'P' && out[2] == b'N' && out[3] == b'G',
        "output must start with PNG signature"
    );
}
