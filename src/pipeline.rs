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

    let files = collect_files(&args.patterns, args.recursive)?;
    if files.is_empty() {
        return Err(AppError::NoMatches);
    }

    let reporter = Reporter {
        dry_run: args.dry_run,
    };

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.job_count())
        .build()
        .map_err(|e| AppError::Io(std::io::Error::other(format!("thread pool: {e}"))))?;

    let dry_run = args.dry_run;
    let no_backup = args.no_backup;
    let quality = args.quality;
    let lossy = args.lossy;
    let no_zopfli = args.no_zopfli;
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
            let outcome =
                optimize_file(path, format, dry_run, no_backup, quality, lossy, no_zopfli);
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
        std::process::exit(1);
    }
    Ok(())
}

fn optimize_file(
    path: &Path,
    format: Format,
    dry_run: bool,
    no_backup: bool,
    quality: Option<u8>,
    lossy: bool,
    no_zopfli: bool,
) -> Outcome {
    let original = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return Outcome::Failed(e.to_string()),
    };
    if original.is_empty() {
        return Outcome::Failed("empty file".into());
    }

    let optimizer = crate::optimize::for_format(format);
    let optimized = match optimizer.optimize(&original, quality, lossy, no_zopfli) {
        Ok(b) => b,
        Err(e) => return Outcome::Failed(e.to_string()),
    };

    if !safety::is_safe_to_write(&original, &optimized, format) {
        return Outcome::Skipped;
    }

    if !dry_run {
        if !no_backup && let Err(e) = backup_if_needed(path, &original) {
            return Outcome::Failed(e.to_string());
        }
        if let Err(e) = write_atomic(path, &optimized) {
            return Outcome::Failed(e.to_string());
        }
    }

    Outcome::Optimized(Stats::from_sizes(
        original.len() as u64,
        optimized.len() as u64,
    ))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
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
