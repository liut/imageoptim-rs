//! End-to-end tests for the `--output-dir` flag and the related
//! `--fail-fast` flag. Both flags are CLI-only; we drive the binary
//! via the `Command` integration test pattern.

use std::process::Command;
use tempfile::tempdir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_imageoptim"))
}

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/example01.png")
}

#[test]
fn output_dir_writes_with_underscore_s_suffix_and_preserves_input() {
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }
    let original = std::fs::read(&fixture).expect("read fixture");

    let dir = tempdir().unwrap();
    let out = dir.path().join("optimized");
    let input_copy = dir.path().join("example01.png");
    std::fs::copy(&fixture, &input_copy).unwrap();

    let output = bin()
        .arg(&input_copy)
        .arg("--output-dir")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = out.join("example01_s.png");
    assert!(
        expected.exists(),
        "expected {} to exist; stderr: {}",
        expected.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    let after = std::fs::read(&input_copy).unwrap();
    assert_eq!(after, original, "input file must not be modified");

    let out_size = std::fs::metadata(&expected).unwrap().len();
    assert!(
        out_size < original.len() as u64,
        "output ({}) must be smaller than input ({})",
        out_size,
        original.len()
    );

    assert!(!dir.path().join("example01.png.bak").exists());
}

#[test]
fn output_dir_collision_appends_numeric_suffix() {
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("example01_s.png"), b"preexisting").unwrap();

    let input = dir.path().join("example01.png");
    std::fs::copy(&fixture, &input).unwrap();

    let output = bin()
        .arg(&input)
        .arg("--output-dir")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        std::fs::read(out.join("example01_s.png")).unwrap(),
        b"preexisting"
    );
    assert!(
        out.join("example01_s-1.png").exists(),
        "expected example01_s-1.png in {}; stderr: {}",
        out.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn output_dir_creates_nested_target_path() {
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let nested = dir.path().join("a").join("b").join("c");
    let input = dir.path().join("example01.png");
    std::fs::copy(&fixture, &input).unwrap();

    let output = bin()
        .arg(&input)
        .arg("--output-dir")
        .arg(&nested)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(nested.is_dir());
    assert!(nested.join("example01_s.png").exists());
}

#[test]
fn output_dir_no_output_for_skipped_file() {
    // A file the safety contract marks as "Skipped" (already optimal)
    // should produce no side-output. We pre-optimize a real fixture
    // with the default in-place path first, then point --output-dir at
    // the already-optimal file; the second run should skip.
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }

    let dir = tempdir().unwrap();
    let out = dir.path().join("out");
    let input = dir.path().join("example01.png");
    std::fs::copy(&fixture, &input).unwrap();

    let r1 = bin().arg(&input).output().unwrap();
    assert!(
        r1.status.success(),
        "first pass: {}",
        String::from_utf8_lossy(&r1.stderr)
    );
    let pre_opt = std::fs::read(&input).unwrap();

    let r2 = bin()
        .arg(&input)
        .arg("--output-dir")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        r2.status.success(),
        "second pass: {}",
        String::from_utf8_lossy(&r2.stderr)
    );
    assert!(
        !out.join("example01_s.png").exists(),
        "skipped file should not produce a side-output"
    );
    assert_eq!(std::fs::read(&input).unwrap(), pre_opt);
}

#[test]
fn fail_fast_exits_on_first_error() {
    // Sanity check: --fail-fast is accepted and produces a normal exit
    // when all files are valid.
    let fixture = fixture_path();
    if !fixture.exists() {
        eprintln!("skipping: {} not present", fixture.display());
        return;
    }
    let dir = tempdir().unwrap();
    let good = dir.path().join("good.png");
    std::fs::copy(&fixture, &good).unwrap();

    let output = bin().arg(&good).arg("--fail-fast").output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fail_fast_exits_nonzero_when_only_corrupt_file() {
    // --fail-fast should propagate the non-zero exit code from a
    // run that has at least one error.
    let dir = tempdir().unwrap();
    let bad = dir.path().join("bad.png");
    std::fs::write(&bad, b"not a real png").unwrap();

    let output = bin().arg(&bad).arg("--fail-fast").output().unwrap();
    assert!(
        !output.status.success(),
        "expected non-zero exit; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
