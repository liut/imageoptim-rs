use std::path::Path;

use crate::detect::Format;

/// Statistics for a single file's optimization result.
#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub original: u64,
    pub optimized: u64,
    pub saved: i64,
    pub percent: f64,
}

impl Stats {
    pub fn from_sizes(original: u64, optimized: u64) -> Self {
        let saved = original as i64 - optimized as i64;
        let percent = if original == 0 {
            0.0
        } else {
            (saved as f64 / original as f64) * 100.0
        };
        Self {
            original,
            optimized,
            saved,
            percent,
        }
    }

    pub const fn skipped() -> Self {
        Self {
            original: 0,
            optimized: 0,
            saved: 0,
            percent: 0.0,
        }
    }
}

#[derive(Debug)]
pub enum Outcome {
    Optimized(Stats),
    Skipped,
    Failed(String),
}

pub struct Reporter {
    pub use_color: bool,
    pub dry_run: bool,
}

impl Reporter {
    pub fn print_file(&self, path: &Path, format: Format, outcome: &Outcome) {
        let label = format!("[{}]", format.name());
        match outcome {
            Outcome::Optimized(s) => {
                if self.dry_run {
                    println!(
                        "  {label} {} {}would save {} ({:.2}%)",
                        path.display(),
                        dim(""),
                        bytes_human(s.saved),
                        s.percent
                    );
                } else {
                    println!(
                        "  {label} {} saved {} ({:.2}%)",
                        path.display(),
                        bytes_human(s.saved.max(0)),
                        s.percent
                    );
                }
            }
            Outcome::Skipped => {
                println!("  {label} {} skipped (already optimal)", path.display());
            }
            Outcome::Failed(err) => {
                eprintln!("  {label} {} failed: {err}", path.display());
            }
        }
    }

    pub fn print_summary(&self, total_files: usize, total_saved: i64, total_pct: f64) {
        println!();
        println!("Processed {total_files} files, saved {} ({:.2}%)", bytes_human(total_saved.max(0)), total_pct);
    }
}

fn dim(s: &str) -> String {
    if s.is_empty() {
        String::new()
    } else {
        format!("\x1b[2m{s}\x1b[0m")
    }
}

pub fn bytes_human(n: i64) -> String {
    let n = n.unsigned_abs();
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}
