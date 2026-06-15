//! End-to-end tests for the `--verbose` and `--summary-only` flags.
//! Drives the binary via `Command::new(env!("CARGO_BIN_EXE_imageoptim"))`,
//! following the same pattern as `tests/max_colors.rs`.

use std::process::Command;
use tempfile::tempdir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_imageoptim"))
}

/// Write a 1x1 RGB PNG to `path`. The full lossy pipeline runs
/// against it (decode → imagequant → oxipng → zopflipng-or-no)
/// and every verbose trace line is emitted, but the file is tiny
/// (~70 B) so each invocation is ~50-100x faster than driving
/// the 2.3 MB `tests/example01.png` fixture. Use this for tests
/// that only need to assert on string presence/absence in the
/// output streams — not for tests that need a real photo for
/// size-delta assertions.
fn write_one_pixel_png(path: &std::path::Path) {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(1, 1, |_, _| Rgb([255, 0, 0]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    std::fs::write(path, &out).unwrap();
}

#[test]
fn verbose_emits_step_details_to_stderr() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("one.png");
    write_one_pixel_png(&input);

    let r = bin()
        .arg(&input)
        .arg("--lossy")
        .arg("--no-zopfli")
        .arg("--verbose")
        .output()
        .unwrap();
    assert!(r.status.success());

    let stderr = String::from_utf8_lossy(&r.stderr);
    // Verbose trace must mention the key steps. We assert on substrings
    // that are stable across oxipng/imagequant version bumps.
    assert!(
        stderr.contains("imagequant"),
        "verbose trace should mention imagequant, got: {stderr}"
    );
    assert!(
        stderr.contains("oxipng preset"),
        "verbose trace should mention the oxipng preset, got: {stderr}"
    );
    assert!(
        stderr.contains("decoded"),
        "verbose trace should report the decoded dimensions, got: {stderr}"
    );
}

#[test]
fn verbose_off_does_not_emit_step_trace() {
    // Same setup as the verbose test, but without --verbose. Stderr
    // should be quiet for the per-step trace. (Errors that go to
    // stderr are unaffected — we're not testing errors here.)
    let dir = tempdir().unwrap();
    let input = dir.path().join("one.png");
    write_one_pixel_png(&input);

    let r = bin()
        .arg(&input)
        .arg("--lossy")
        .arg("--no-zopfli")
        .output()
        .unwrap();
    assert!(r.status.success());

    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        !stderr.contains("imageoptim: png"),
        "non-verbose run should not emit the per-step trace, got: {stderr}"
    );
    assert!(
        !stderr.contains("imagequant q="),
        "non-verbose run should not mention the imagequant step, got: {stderr}"
    );
}

#[test]
fn summary_only_suppresses_per_file_lines() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("one.png");
    write_one_pixel_png(&input);

    let r = bin()
        .arg(&input)
        .arg("--lossy")
        .arg("--no-zopfli")
        .arg("--summary-only")
        .output()
        .unwrap();
    assert!(r.status.success());

    let stdout = String::from_utf8_lossy(&r.stdout);
    // Per-file line: `[PNG] ... saved ...`
    assert!(
        !stdout.contains("[PNG]"),
        "summary-only should suppress per-file lines, got: {stdout}"
    );
    // Summary still printed.
    assert!(
        stdout.contains("Processed"),
        "summary-only should still print the summary, got: {stdout}"
    );
}

#[test]
fn summary_only_still_reports_failures() {
    // Build a fake "png" that is not actually a PNG. The optimizer
    // will fail; --summary-only should still surface the error to
    // stderr (the per-file skipped/optimized line is suppressed, but
    // the error path is preserved).
    let dir = tempdir().unwrap();
    let input = dir.path().join("not-a-png.png");
    std::fs::write(&input, b"definitely not a png").unwrap();

    let r = bin().arg(&input).arg("--summary-only").output().unwrap();
    // Exit code is 1 (any file failed); the per-file line is absent
    // from stdout, and the error surfaces on stderr.
    assert!(!r.status.success());

    let stdout = String::from_utf8_lossy(&r.stdout);
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        !stdout.contains("[PNG]"),
        "summary-only should suppress the failed per-file line on stdout, got: {stdout}"
    );
    assert!(
        stderr.contains("failed"),
        "summary-only should still report the failure on stderr, got: {stderr}"
    );
}
