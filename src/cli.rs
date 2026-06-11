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

    /// Allow lossy optimization for PNG (palette quantization). Off by
    /// default. Up to 256 colors by default; pass `--max-colors` to cap
    /// the palette at a smaller size. The output is still required to
    /// be smaller than the input and to decode as a valid PNG.
    #[arg(long)]
    pub lossy: bool,

    /// Cap the palette size used by `--lossy` PNG at `<N>` colors
    /// (range 2..=256). Requires `--lossy`; ignored on non-PNG inputs.
    /// Default is 256, which is imagequant's built-in maximum.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(2..=256))]
    pub max_colors: Option<u32>,

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

    /// oxipng preset to use for the PNG inner step (range 0..=6).
    /// Lower numbers are faster but produce larger output; higher
    /// numbers are slower but compress more. Defaults to 3 for
    /// lossless PNG and to 6 for `--lossy` PNG — pass this flag to
    /// override either mode. Ignored for non-PNG inputs.
    #[arg(long, value_name = "0-6", value_parser = clap::value_parser!(u8).range(0..=6))]
    pub png_optimization_level: Option<u8>,

    /// Number of parallel workers.
    #[arg(short, long, value_name = "N")]
    pub jobs: Option<usize>,

    /// Stop processing on the first per-file error. By default, every
    /// file is processed and a per-file error summary is printed at the
    /// end (the exit code is still 1 if any file failed). Pass
    /// `--fail-fast` to short-circuit and exit immediately on the first
    /// error. Useful in CI pipelines where any failure should stop the
    /// build.
    #[arg(long)]
    pub fail_fast: bool,
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
