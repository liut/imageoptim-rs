use clap::Parser;

/// Cross-platform image optimization CLI.
#[derive(Debug, Parser)]
#[command(name = "imageoptim", version, about)]
pub struct Args {
    /// File paths or glob patterns (e.g. `*.png`, `assets/**/*.jpg`).
    #[arg(value_name = "PATTERN")]
    pub patterns: Vec<String>,

    /// Recurse into directories.
    #[arg(short, long)]
    pub recursive: bool,

    /// Show what would be done without modifying any files.
    #[arg(long)]
    pub dry_run: bool,

    /// Disable ANSI color output.
    #[arg(long)]
    pub no_color: bool,

    /// Quality for lossy formats (0-100). Omit for lossless.
    #[arg(short, long, value_name = "0-100")]
    pub quality: Option<u8>,

    /// Number of parallel workers.
    #[arg(short, long, value_name = "N")]
    pub jobs: Option<usize>,
}

impl Args {
    pub fn job_count(&self) -> usize {
        self.jobs.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        })
    }
}
