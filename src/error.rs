use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("no input files or patterns provided")]
    NoInput,

    #[error("no files matched the given patterns")]
    NoMatches,

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("file `{path}` is empty")]
    EmptyFile { path: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("optimization failed for `{path}`: {source}")]
    Optimize {
        path: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("glob pattern `{pattern}` is invalid: {source}")]
    Glob {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },

    #[error("glob expansion failed: {0}")]
    GlobExpansion(#[from] glob::GlobError),
}
