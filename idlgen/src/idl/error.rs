use std::fmt;
use std::path::PathBuf;

/// Why an IDL could not be read.
#[derive(Debug)]
pub enum IdlError {
    /// The IDL file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The text is not valid JSON.
    Json(serde_json::Error),
    /// The JSON is not a valid IDL, or uses an unsupported construct.
    Invalid {
        /// Where in the JSON, such as `instructions[3].args[1].type`.
        path: String,
        message: String,
    },
}

impl IdlError {
    pub(crate) fn invalid(path: impl Into<String>, message: impl Into<String>) -> IdlError {
        IdlError::Invalid {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for IdlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdlError::Io { path, source } => write!(f, "reading {}: {}", path.display(), source),
            IdlError::Json(err) => write!(f, "invalid JSON: {err}"),
            IdlError::Invalid { path, message } if path.is_empty() => write!(f, "{message}"),
            IdlError::Invalid { path, message } => write!(f, "{path}: {message}"),
        }
    }
}

impl std::error::Error for IdlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IdlError::Io { source, .. } => Some(source),
            IdlError::Json(err) => Some(err),
            IdlError::Invalid { .. } => None,
        }
    }
}
