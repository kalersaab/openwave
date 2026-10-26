use std::fmt;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, OpenWaveError>;

/// Errors surfaced by the media pipeline and reported by the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenWaveError {
    /// The file does not contain a media format OpenWave can identify.
    UnsupportedFormat {
        path: Option<PathBuf>,
        details: String,
    },
    /// The file could not be read while detecting or opening it.
    Io {
        path: Option<PathBuf>,
        details: String,
    },
}

impl OpenWaveError {
    pub fn unsupported_format(details: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            path: None,
            details: details.into(),
        }
    }

    pub fn unsupported_format_for(path: &Path, details: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            path: Some(path.to_path_buf()),
            details: details.into(),
        }
    }

    pub fn io(path: &Path, error: &std::io::Error) -> Self {
        Self::Io {
            path: Some(path.to_path_buf()),
            details: error.to_string(),
        }
    }

    pub fn is_unsupported_format(&self) -> bool {
        matches!(self, Self::UnsupportedFormat { .. })
    }

    pub fn message(&self) -> &str {
        match self {
            Self::UnsupportedFormat { details, .. } | Self::Io { details, .. } => details,
        }
    }

    /// Attach a path to an error produced while probing in-memory data.
    pub fn with_path(self, path: &Path) -> Self {
        match self {
            Self::UnsupportedFormat {
                path: None,
                details,
            } => Self::UnsupportedFormat {
                path: Some(path.to_path_buf()),
                details,
            },
            Self::Io {
                path: None,
                details,
            } => Self::Io {
                path: Some(path.to_path_buf()),
                details,
            },
            other => other,
        }
    }
}

impl fmt::Display for OpenWaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat {
                path: Some(path),
                details,
            } => write!(
                f,
                "unsupported media format for '{}': {details}",
                path.display()
            ),
            Self::UnsupportedFormat {
                path: None,
                details,
            } => write!(f, "unsupported media format: {details}"),
            Self::Io {
                path: Some(path),
                details,
            } => write!(f, "I/O error while reading '{}': {details}", path.display()),
            Self::Io {
                path: None,
                details,
            } => write!(f, "I/O error: {details}"),
        }
    }
}

impl std::error::Error for OpenWaveError {}

impl From<std::io::Error> for OpenWaveError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            path: None,
            details: error.to_string(),
        }
    }
}

/// Central sink for errors that should be surfaced to the user or a log.
#[derive(Debug, Default)]
pub struct ErrorHandler {
    reported: Vec<OpenWaveError>,
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Report an error to stderr and retain it for diagnostics.
    pub fn report(&mut self, error: &OpenWaveError) {
        eprintln!("[OpenWave] {error}");
        self.reported.push(error.clone());
    }

    pub fn reported(&self) -> &[OpenWaveError] {
        &self.reported
    }

    pub fn last_reported(&self) -> Option<&OpenWaveError> {
        self.reported.last()
    }

    pub fn clear(&mut self) {
        self.reported.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.reported.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_format_error_has_expected_message() {
        let error = OpenWaveError::unsupported_format("unrecognised signature");
        assert!(error.is_unsupported_format());
        assert!(error.to_string().contains("unsupported media format"));
    }

    #[test]
    fn error_handler_records_reported_errors() {
        let mut handler = ErrorHandler::new();
        let error =
            OpenWaveError::unsupported_format_for(Path::new("clip.bin"), "unrecognised signature");

        handler.report(&error);

        assert_eq!(handler.reported(), &[error]);
        assert_eq!(handler.last_reported(), Some(&handler.reported()[0]));
    }
}
