use std::fmt;
use std::path::{Path, PathBuf};

use crate::media::decoder::Codec;

pub type Result<T> = std::result::Result<T, OpenWaveError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenWaveError {
    UnsupportedFormat {
        path: Option<PathBuf>,
        details: String,
    },
    Io {
        path: Option<PathBuf>,
        details: String,
    },
    Playback {
        path: Option<PathBuf>,
        details: String,
    },
    MissingCodec {
        path: Option<PathBuf>,
        codec: Codec,
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

    pub fn missing_codec(path: Option<&Path>, codec: Codec) -> Self {
        Self::MissingCodec {
            path: path.map(Path::to_path_buf),
            codec,
        }
    }

    pub fn playback(path: Option<&Path>, details: impl Into<String>) -> Self {
        Self::Playback {
            path: path.map(Path::to_path_buf),
            details: details.into(),
        }
    }

    pub fn is_unsupported_format(&self) -> bool {
        matches!(self, Self::UnsupportedFormat { .. })
    }

    pub fn message(&self) -> &str {
        match self {
            Self::UnsupportedFormat { details, .. }
            | Self::Io { details, .. }
            | Self::Playback { details, .. } => details,
            Self::MissingCodec { .. } => "required codec is unavailable",
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
            Self::Playback {
                path: None,
                details,
            } => Self::Playback {
                path: Some(path.to_path_buf()),
                details,
            },
            Self::MissingCodec { path: None, codec } => Self::MissingCodec {
                path: Some(path.to_path_buf()),
                codec,
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
            Self::Playback {
                path: Some(path),
                details,
            } => write!(f, "playback error for '{}': {details}", path.display()),
            Self::Playback {
                path: None,
                details,
            } => write!(f, "playback error: {details}"),
            Self::MissingCodec {
                path: Some(path),
                codec,
            } => write!(
                f,
                "missing codec '{codec}' required to decode '{}'",
                path.display()
            ),
            Self::MissingCodec { path: None, codec } => {
                write!(f, "missing codec '{codec}' required to decode media")
            }
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

#[derive(Debug, Default)]
pub struct ErrorHandler {
    reported: Vec<OpenWaveError>,
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self::default()
    }

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

    #[test]
    fn missing_codec_error_includes_codec_and_path() {
        let error = OpenWaveError::missing_codec(
            Some(Path::new("clip.mp4")),
            Codec::Video(crate::media::decoder::VideoCodec::H265),
        );

        assert!(error.to_string().contains("missing codec 'video/H.265'"));
        assert!(error.to_string().contains("clip.mp4"));
        assert_eq!(error.clone().with_path(Path::new("other.mp4")), error);
        assert_eq!(
            OpenWaveError::missing_codec(
                None,
                Codec::Video(crate::media::decoder::VideoCodec::Av1)
            )
            .with_path(Path::new("clip.mkv")),
            OpenWaveError::missing_codec(
                Some(Path::new("clip.mkv")),
                Codec::Video(crate::media::decoder::VideoCodec::Av1)
            )
        );
    }
}
