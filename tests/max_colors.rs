//! End-to-end tests for the `--max-colors` flag. Drives the binary
//! via `Command::new(env!("CARGO_BIN_EXE_imageoptim"))`, following the
//! same pattern as `tests/output_dir.rs`.

use std::process::Command;
use tempfile::tempdir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_imageoptim"))
}

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example01.png")
}

#[test]
fn max_colors_reduces_output_at_small_n() {
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let at_256 = dir.path().join("at_256.png");
    let at_16 = dir.path().join("at_16.png");
    std::fs::copy(&fixture, &at_256).unwrap();
    std::fs::copy(&fixture, &at_16).unwrap();

    let r256 = bin()
        .arg(&at_256)
        .arg("--lossy")
        .arg("--max-colors")
        .arg("256")
        .output()
        .unwrap();
    assert!(r256.status.success());

    let r16 = bin()
        .arg(&at_16)
        .arg("--lossy")
        .arg("--max-colors")
        .arg("16")
        .output()
        .unwrap();

    let size_256 = std::fs::metadata(&at_256).unwrap().len();
    let size_16 = std::fs::metadata(&at_16).unwrap().len();

    // Two acceptable outcomes on a real photo:
    //
    //   1. imagequant at N=16 hits the 80-100 quality target and
    //      the file is shrunk below the N=256 size. Strict
    //      assertion holds.
    //
    //   2. imagequant cannot meet the quality target with 16
    //      colors (e.g. for a per-pixel-noisy synthetic fixture
    //      that exceeds the noise budget imagequant allows), the
    //      binary exits 1, and the safety contract leaves the
    //      file unchanged. `size_16` then equals the original
    //      size — strictly greater than `size_256`, which is
    //      expected and acceptable.
    //
    // The test fails only if N=16 produces a successful exit
    // AND the output is not strictly smaller than N=256, which
    // would be a real regression.
    if r16.status.success() {
        assert!(
            size_16 < size_256,
            "N=16 should be strictly smaller than N=256 when both succeed (was {size_16} vs {size_256})"
        );
    } else {
        // Failure path: input must be unchanged (safety contract).
        let original_size = std::fs::metadata(&fixture).unwrap().len();
        assert_eq!(
            size_16, original_size,
            "N=16 failed but the input was modified (was {size_16}, expected {original_size})"
        );
    }
}

#[test]
fn max_colors_256_matches_default() {
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let with_flag = dir.path().join("with_flag.png");
    let without_flag = dir.path().join("without_flag.png");
    std::fs::copy(&fixture, &with_flag).unwrap();
    std::fs::copy(&fixture, &without_flag).unwrap();

    let r1 = bin()
        .arg(&with_flag)
        .arg("--lossy")
        .arg("--max-colors")
        .arg("256")
        .output()
        .unwrap();
    assert!(r1.status.success());

    let r2 = bin().arg(&without_flag).arg("--lossy").output().unwrap();
    assert!(r2.status.success());

    let s_flag = std::fs::metadata(&with_flag).unwrap().len();
    let s_default = std::fs::metadata(&without_flag).unwrap().len();
    assert_eq!(
        s_flag, s_default,
        "explicit --max-colors 256 must match the default behavior"
    );
}

#[test]
fn max_colors_clamps_at_two() {
    // The smallest valid value (2). At the quality constraint (80-100)
    // imagequant may simply refuse to produce a 2-color palette for a
    // real photo. The pipeline surfaces that as a per-file Failed
    // outcome (exit code 1). What matters here is that the input is
    // not corrupted, and the tool reports a clear outcome rather than
    // silently overwriting with garbage.
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let input = dir.path().join("example01.png");
    std::fs::copy(&fixture, &input).unwrap();
    let original = std::fs::read(&input).unwrap();

    let r = bin()
        .arg(&input)
        .arg("--lossy")
        .arg("--max-colors")
        .arg("2")
        .output()
        .unwrap();

    // Exit code 0 means the tool optimized or skipped; 1 means a
    // per-file failure (imagequant refused the 2-color constraint).
    // Both are acceptable: the safety contract holds either way
    // (no corrupted overwrites).
    let stderr = String::from_utf8_lossy(&r.stderr);
    let stdout = String::from_utf8_lossy(&r.stdout);
    let has_outcome =
        stdout.contains("saved") || stdout.contains("skipped") || stderr.contains("failed");
    assert!(
        has_outcome,
        "expected a clear outcome (saved/skipped/failed) for max-colors 2; stdout={stdout}; stderr={stderr}"
    );

    // Critical: the input file is never modified, regardless of which
    // outcome the tool reported.
    assert_eq!(std::fs::read(&input).unwrap(), original);
}

#[test]
fn max_colors_without_lossy_errors() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("foo.png");
    // Minimal valid PNG (1x1 RGB)
    std::fs::write(
        &input,
        [
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x5b, 0x6d, 0x4b,
            0x4a, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ],
    )
    .unwrap();

    let r = bin()
        .arg(&input)
        .arg("--max-colors")
        .arg("64")
        .output()
        .unwrap();
    assert!(!r.status.success(), "should have failed without --lossy");
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("--max-colors requires --lossy"),
        "stderr should mention the missing flag, got: {stderr}"
    );
}

#[test]
fn max_colors_above_range_rejected() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("foo.png");
    std::fs::write(&input, b"not a real png").unwrap();
    let original = std::fs::read(&input).unwrap();

    let r = bin()
        .arg(&input)
        .arg("--lossy")
        .arg("--max-colors")
        .arg("999")
        .output()
        .unwrap();
    assert!(!r.status.success(), "should fail at parse time");
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("not in 2..=256"),
        "stderr should mention the range, got: {stderr}"
    );
    // Input not modified.
    assert_eq!(std::fs::read(&input).unwrap(), original);
}
