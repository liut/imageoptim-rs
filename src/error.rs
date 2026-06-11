use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("no input files or patterns provided")]
    NoInput,

    #[error("no files matched the given patterns")]
    NoMatches,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("glob pattern `{pattern}` is invalid: {source}")]
    Glob {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },

    #[error("glob expansion failed: {0}")]
    GlobExpansion(#[from] glob::GlobError),

    #[error("--max-colors requires --lossy")]
    MaxColorsRequiresLossy,

    #[error("one or more files failed to optimize")]
    AnyFileFailed,
}
