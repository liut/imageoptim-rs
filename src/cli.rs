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

    /// Skip creating `<path>.bak` before overwriting. The optimized file
    /// is still written only when the safety contract holds.
    #[arg(long)]
    pub no_backup: bool,

    /// Allow lossy optimization for PNG (palette quantization to 256
    /// colors). Off by default. The output is still required to be
    /// smaller than the input and to decode as a valid PNG.
    #[arg(long)]
    pub lossy: bool,

    /// Disable the optional `zopflipng` post-pass in the lossy PNG
    /// pipeline. On by default — the lossy pipeline auto-detects
    /// `zopflipng` in `$PATH` and runs it for additional compression if
    /// found. Pass `--no-zopfli` to skip that step (e.g. on systems
    /// where `zopflipng` is not installed). Ignored if `--lossy` is not set.
    #[arg(long)]
    pub no_zopfli: bool,

    /// Write optimized files into `<DIR>` instead of overwriting the
    /// inputs in place. Each output is named `<stem>_s<ext>`; if a file
    /// with that name already exists, a numeric suffix (`-1`, `-2`, ...)
    /// is appended to avoid clobbering. The original input is left
    /// untouched, so `--no-backup` is implicit when this flag is set.
    /// The directory is created if it does not already exist.
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<std::path::PathBuf>,

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
