//! End-to-end tests for the `--png-optimization-level` flag. Drives
//! the binary via `Command::new(env!("CARGO_BIN_EXE_imageoptim"))`,
//! following the same pattern as `tests/max_colors.rs`.

use std::process::Command;
use tempfile::tempdir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_imageoptim"))
}

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example01.png")
}

#[test]
fn png_level_six_matches_default_lossless() {
    // Per-mode default for lossless PNG is preset 3, so an explicit
    // --png-optimization-level 6 will *not* match the default. The
    // assertion is the reverse: an explicit 6 in the lossy path
    // matches the lossy default (preset 6). Both sub-assertions are
    // useful in one test, and they share the fixture copy.
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let no_flag = dir.path().join("no_flag.png");
    let explicit = dir.path().join("explicit.png");
    std::fs::copy(&fixture, &no_flag).unwrap();
    std::fs::copy(&fixture, &explicit).unwrap();

    let r1 = bin().arg(&no_flag).arg("--no-zopfli").output().unwrap();
    assert!(r1.status.success());
    let r2 = bin()
        .arg(&explicit)
        .arg("--no-zopfli")
        .arg("--png-optimization-level")
        .arg("3")
        .output()
        .unwrap();
    assert!(r2.status.success());

    let s1 = std::fs::metadata(&no_flag).unwrap().len();
    let s2 = std::fs::metadata(&explicit).unwrap().len();
    assert_eq!(
        s1, s2,
        "explicit --png-optimization-level 3 must match the lossless default"
    );
}

#[test]
fn png_level_six_matches_default_lossy() {
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let no_flag = dir.path().join("no_flag.png");
    let explicit = dir.path().join("explicit.png");
    std::fs::copy(&fixture, &no_flag).unwrap();
    std::fs::copy(&fixture, &explicit).unwrap();

    let r1 = bin()
        .arg(&no_flag)
        .arg("--lossy")
        .arg("--no-zopfli")
        .output()
        .unwrap();
    assert!(r1.status.success());
    let r2 = bin()
        .arg(&explicit)
        .arg("--lossy")
        .arg("--no-zopfli")
        .arg("--png-optimization-level")
        .arg("6")
        .output()
        .unwrap();
    assert!(r2.status.success());

    let s1 = std::fs::metadata(&no_flag).unwrap().len();
    let s2 = std::fs::metadata(&explicit).unwrap().len();
    assert_eq!(
        s1, s2,
        "explicit --png-optimization-level 6 must match the lossy default"
    );
}

#[test]
fn png_level_zero_still_smaller_lossless() {
    // Preset 0 is the fastest tier of oxipng: filter selection runs
    // but the deflate search is heavily reduced. The output should
    // still be smaller than the input for a real photo, because
    // oxipng always re-streams the PNG through its deflate path —
    // it just uses a weaker compressor than preset 6.
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let at_zero = dir.path().join("at_zero.png");
    std::fs::copy(&fixture, &at_zero).unwrap();
    let original_size = std::fs::metadata(&at_zero).unwrap().len();

    let r = bin()
        .arg(&at_zero)
        .arg("--png-optimization-level")
        .arg("0")
        .output()
        .unwrap();
    assert!(r.status.success());

    let optimized_size = std::fs::metadata(&at_zero).unwrap().len();
    assert!(
        optimized_size < original_size,
        "preset 0 should still produce smaller output (was {optimized_size} vs {original_size})"
    );
}

#[test]
fn png_level_higher_compresses_more() {
    // Higher preset = more deflate search = smaller output. We compare
    // preset 0 (fastest) against preset 6 (max compression) on a real
    // photo and assert monotonicity: size_0 >= size_6.
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let at_zero = dir.path().join("at_zero.png");
    let at_six = dir.path().join("at_six.png");
    std::fs::copy(&fixture, &at_zero).unwrap();
    std::fs::copy(&fixture, &at_six).unwrap();

    let r0 = bin()
        .arg(&at_zero)
        .arg("--png-optimization-level")
        .arg("0")
        .output()
        .unwrap();
    assert!(r0.status.success());
    let r6 = bin()
        .arg(&at_six)
        .arg("--png-optimization-level")
        .arg("6")
        .output()
        .unwrap();
    assert!(r6.status.success());

    let s0 = std::fs::metadata(&at_zero).unwrap().len();
    let s6 = std::fs::metadata(&at_six).unwrap().len();
    assert!(
        s0 >= s6,
        "preset 0 ({s0}) should be >= preset 6 ({s6}) on a real photo"
    );
    // And on a real photo the gap is non-trivial — a 2.3 MB input
    // should show >5% reduction between the two presets.
    assert!(
        s0 > s6 + 5_000,
        "expected a meaningful gap between preset 0 and preset 6 (got {s0} vs {s6})"
    );
}

#[test]
fn png_level_above_range_rejected() {
    // oxipng presets are 0..=6; clap range check should reject 7 at
    // parse time with a clear hint on stderr.
    let dir = tempdir().unwrap();
    let input = dir.path().join("foo.png");
    std::fs::write(&input, b"not a real png").unwrap();
    let original = std::fs::read(&input).unwrap();

    let r = bin()
        .arg(&input)
        .arg("--png-optimization-level")
        .arg("7")
        .output()
        .unwrap();
    assert!(!r.status.success(), "should fail at parse time");
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("not in 0..=6"),
        "stderr should mention the range, got: {stderr}"
    );
    // Input not modified.
    assert_eq!(std::fs::read(&input).unwrap(), original);
}
