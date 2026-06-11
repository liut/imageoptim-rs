use crate::cli::Args;
use crate::detect::Format;
use crate::error::AppError;
use crate::report::{Outcome, Reporter, Stats};
use crate::safety;

use indicatif::ParallelProgressIterator;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn run(args: Args) -> Result<(), AppError> {
    if args.patterns.is_empty() {
        return Err(AppError::NoInput);
    }

    // --max-colors only applies to the --lossy PNG path. Reject
    // mismatches up front so a confused invocation produces a single
    // clear error instead of a per-file failure cascade.
    if args.max_colors.is_some() && !args.lossy {
        return Err(AppError::MaxColorsRequiresLossy);
    }

    let files = collect_files(&args.patterns, args.recursive)?;
    if files.is_empty() {
        return Err(AppError::NoMatches);
    }

    // Create the output directory up front so per-file writes don't race
    // to mkdir the same path. Only relevant when --output-dir is set.
    if let Some(dir) = args.output_dir.as_ref() {
        std::fs::create_dir_all(dir).map_err(AppError::Io)?;
    }

    let reporter = Reporter {
        dry_run: args.dry_run,
    };

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.job_count())
        .build()
        .map_err(|e| AppError::Io(std::io::Error::other(format!("thread pool: {e}"))))?;

    let dry_run = args.dry_run;
    // --output-dir means the input is never touched, so --no-backup is implicit.
    let no_backup = args.no_backup || args.output_dir.is_some();
    let quality = args.quality;
    let lossy = args.lossy;
    let no_zopfli = args.no_zopfli;
    let max_colors = args.max_colors;
    let png_level = args.png_optimization_level;
    let fail_fast = args.fail_fast;
    let output_dir = args.output_dir.clone();
    let show_progress = !dry_run && std::io::IsTerminal::is_terminal(&std::io::stderr());
    let pb = if show_progress {
        let pb = indicatif::ProgressBar::new(files.len() as u64);
        pb.set_style(
            indicatif::ProgressStyle::with_template(
                "{spinner:.green} [{bar:30.cyan/blue}] {pos}/{len} {msg}",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let results: Vec<(PathBuf, Format, Outcome)> = pool.install(|| {
        let iter = files.par_iter().map(|path| {
            let format = match Format::from_path(path) {
                Some(f) => f,
                None => {
                    return (
                        path.clone(),
                        Format::Png,
                        Outcome::Failed(format!("unsupported format: {}", path.display())),
                    );
                }
            };
            let outcome = optimize_file(
                path,
                format,
                dry_run,
                no_backup,
                quality,
                lossy,
                no_zopfli,
                max_colors,
                png_level,
                output_dir.as_deref(),
            );
            (path.clone(), format, outcome)
        });
        if let Some(pb) = pb.clone() {
            iter.progress_with(pb).collect()
        } else {
            iter.collect()
        }
    });

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    let mut total_files = 0;
    let mut total_saved: i64 = 0;
    let mut total_original: u64 = 0;
    let mut any_failed = false;

    for (path, format, outcome) in &results {
        reporter.print_file(path, *format, outcome);
        match outcome {
            Outcome::Optimized(s) => {
                total_files += 1;
                total_saved += s.saved;
                total_original += s.original;
                if s.saved < 0 {
                    any_failed = true;
                }
            }
            Outcome::Skipped => {
                total_files += 1;
            }
            Outcome::Failed(_) => any_failed = true,
        }
    }

    let total_pct = if total_original == 0 {
        0.0
    } else {
        (total_saved as f64 / total_original as f64) * 100.0
    };
    reporter.print_summary(total_files, total_saved, total_pct);

    if any_failed {
        if fail_fast {
            // Exit immediately. The summary still printed (the partial
            // report is more useful than a bare exit code), and the
            // exit code distinguishes partial-failure from all-success
            // for the shell.
            std::process::exit(1);
        }
        return Err(AppError::AnyFileFailed);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn optimize_file(
    path: &Path,
    format: Format,
    dry_run: bool,
    no_backup: bool,
    quality: Option<u8>,
    lossy: bool,
    no_zopfli: bool,
    max_colors: Option<u32>,
    png_level: Option<u8>,
    output_dir: Option<&Path>,
) -> Outcome {
    let original = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return Outcome::Failed(e.to_string()),
    };
    if original.is_empty() {
        return Outcome::Failed("empty file".into());
    }

    let optimizer = crate::optimize::for_format(format);
    let optimized = match optimizer.optimize(
        &original,
        quality,
        lossy,
        no_zopfli,
        max_colors,
        png_level,
    ) {
        Ok(b) => b,
        Err(e) => return Outcome::Failed(e.to_string()),
    };

    if !safety::is_safe_to_write(&original, &optimized, format) {
        return Outcome::Skipped;
    }

    // Determine the write target. With --output-dir, write to
    // <output_dir>/<stem>_s<ext> (with collision suffix -1, -2, ...).
    // Otherwise overwrite the input in place via a temp file + rename.
    let (target, via_tmp) = match output_dir {
        Some(dir) => {
            let target = match unique_output_path(dir, path) {
                Ok(t) => t,
                Err(e) => return Outcome::Failed(e.to_string()),
            };
            (target, false)
        }
        None => (path.to_path_buf(), true),
    };

    if !dry_run {
        if !no_backup && let Err(e) = backup_if_needed(path, &original) {
            return Outcome::Failed(e.to_string());
        }
        let write_result = if via_tmp {
            write_atomic(&target, &optimized)
        } else {
            std::fs::write(&target, &optimized)
        };
        if let Err(e) = write_result {
            return Outcome::Failed(e.to_string());
        }
    }

    Outcome::Optimized(Stats::from_sizes(
        original.len() as u64,
        optimized.len() as u64,
    ))
}

/// Build the output path `<dir>/<stem>_s<ext>`, with a `-N` numeric
/// suffix when `<stem>_s<ext>` is already taken. The numbering starts
/// at `-1` and increments until a free path is found.
fn unique_output_path(dir: &Path, input: &Path) -> std::io::Result<PathBuf> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file stem"))?;
    let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("");

    let candidate = dir.join(format!("{stem}_s.{ext}"));
    if !candidate.exists() {
        return Ok(candidate);
    }
    for n in 1..=u32::MAX {
        let c = dir.join(format!("{stem}_s-{n}.{ext}"));
        if !c.exists() {
            return Ok(c);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "exhausted suffixes",
    ))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));
    std::fs::write(&tmp, bytes)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Best-effort cleanup of the temp file we wrote. If the
            // cleanup itself fails, we still surface the original rename
            // error — the user can find the .tmp later or use a future
            // --clean-tmp sweep.
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Copy the original file to `<path>.bak` if a backup does not already exist.
/// Subsequent runs do not overwrite an existing `.bak`, so the first-run
/// backup is always the pre-optimization original.
fn backup_if_needed(path: &Path, original: &[u8]) -> std::io::Result<()> {
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    let bak = PathBuf::from(bak);
    if bak.exists() {
        return Ok(());
    }
    std::fs::write(&bak, original)
}

fn collect_files(patterns: &[String], recursive: bool) -> Result<Vec<PathBuf>, AppError> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();

    for pattern in patterns {
        if let Some(path) = glob_literal(pattern) {
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.exists() {
                files.push(path);
                continue;
            }
        }
        for entry in glob::glob(pattern).map_err(|e| AppError::Glob {
            pattern: pattern.clone(),
            source: e,
        })? {
            let p = entry?;
            if p.is_dir() {
                if recursive {
                    dirs.push(p);
                }
            } else if p.is_file() {
                files.push(p);
            }
        }
    }

    if recursive {
        for dir in dirs {
            for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn glob_literal(s: &str) -> Option<PathBuf> {
    if s.contains('*') || s.contains('?') || s.contains('[') {
        None
    } else {
        Some(PathBuf::from(s))
    }
}
