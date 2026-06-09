use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_imageoptim"))
}

fn make_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(16, 16, |x, y| Rgb([(x * 16) as u8, (y * 16) as u8, 64]));
    let mut out = Vec::new();
    let dyn_img = image::DynamicImage::ImageRgb8(img);
    dyn_img
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

#[test]
fn exit_code_zero_on_success() {
    let dir = tempfile::tempdir().unwrap();
    let png_path = dir.path().join("test.png");
    std::fs::write(&png_path, make_png()).unwrap();

    let output = bin().arg(&png_path).output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[PNG]"),
        "expected PNG label, got: {stdout}"
    );
    assert!(
        stdout.contains("saved"),
        "expected savings report, got: {stdout}"
    );
}

#[test]
fn dry_run_does_not_modify_file() {
    let dir = tempfile::tempdir().unwrap();
    let png_path = dir.path().join("test.png");
    let original_bytes = make_png();
    std::fs::write(&png_path, &original_bytes).unwrap();
    let mtime_before = std::fs::metadata(&png_path).unwrap().modified().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(50));

    let output = bin().arg(&png_path).arg("--dry-run").output().unwrap();
    assert!(output.status.success());

    let after_bytes = std::fs::read(&png_path).unwrap();
    assert_eq!(
        after_bytes, original_bytes,
        "file content changed in dry-run"
    );

    let mtime_after = std::fs::metadata(&png_path).unwrap().modified().unwrap();
    assert_eq!(mtime_before, mtime_after, "mtime changed in dry-run");
}

#[test]
fn corrupt_file_does_not_overwrite_original() {
    let dir = tempfile::tempdir().unwrap();
    let png_path = dir.path().join("corrupt.png");
    let good_path = dir.path().join("good.png");
    let original_corrupt = b"this is not a valid PNG file at all".to_vec();
    std::fs::write(&png_path, &original_corrupt).unwrap();
    std::fs::write(&good_path, make_png()).unwrap();

    let output = bin().arg(&png_path).arg(&good_path).output().unwrap();
    assert!(
        !output.status.success(),
        "expected non-zero exit when a file fails"
    );

    let after = std::fs::read(&png_path).unwrap();
    assert_eq!(after, original_corrupt, "corrupt file was overwritten!");
}

#[test]
fn no_matches_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let pattern = dir
        .path()
        .join("nonexistent-*.png")
        .to_string_lossy()
        .to_string();
    let output = bin().arg(&pattern).output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn glob_finds_pngs_in_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.png"), make_png()).unwrap();
    std::fs::write(dir.path().join("b.png"), make_png()).unwrap();
    std::fs::write(dir.path().join("c.txt"), b"text").unwrap();

    let pattern = dir.path().join("*.png").to_string_lossy().to_string();
    let output = bin().arg(&pattern).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let png_count = stdout.matches("[PNG]").count();
    assert_eq!(
        png_count, 2,
        "expected 2 PNG reports, got {png_count}: {stdout}"
    );
}

#[test]
fn recursive_flag_walks_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("nested.png"), make_png()).unwrap();

    let output = bin()
        .arg(dir.path().to_str().unwrap())
        .arg("-r")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("nested.png"),
        "recursive walk missed file: {stdout}"
    );
}

#[test]
fn jobs_flag_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let png_path = dir.path().join("test.png");
    std::fs::write(&png_path, make_png()).unwrap();

    let output = bin().arg(&png_path).arg("-j").arg("1").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn quality_flag_changes_jpeg_output_size() {
    let dir = tempfile::tempdir().unwrap();
    let high = dir.path().join("high.jpg");
    let low = dir.path().join("low.jpg");
    let jpg = make_jpeg();
    std::fs::write(&high, &jpg).unwrap();
    std::fs::write(&low, &jpg).unwrap();

    let h = bin()
        .arg(&high)
        .arg("-q")
        .arg("95")
        .arg("--no-backup")
        .output()
        .unwrap();
    let l = bin()
        .arg(&low)
        .arg("-q")
        .arg("20")
        .arg("--no-backup")
        .output()
        .unwrap();
    assert!(h.status.success() && l.status.success());

    let h_size = std::fs::metadata(&high).unwrap().len();
    let l_size = std::fs::metadata(&low).unwrap().len();
    assert!(
        l_size < h_size,
        "q=20 ({l_size}) must be smaller than q=95 ({h_size})"
    );
}

fn make_jpeg() -> Vec<u8> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_fn(64, 64, |x, y| Rgb([(x * 4) as u8, (y * 4) as u8, 128]));
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut out),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    out
}
