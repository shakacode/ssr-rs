use std::{fmt, io, net::AddrParseError};

#[derive(Debug)]
pub enum InitializationError {
    InvalidAddr(AddrParseError),
    InvalidJsWorkerPath(io::Error),
    InvalidGlobalJsRendererPath(io::Error),
    SpawnNodeProcessError(io::Error),
}

impl From<AddrParseError> for InitializationError {
    fn from(err: AddrParseError) -> Self {
        Self::InvalidAddr(err)
    }
}

impl From<io::Error> for InitializationError {
    fn from(err: io::Error) -> Self {
        Self::SpawnNodeProcessError(err)
    }
}

impl fmt::Display for InitializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddr(err) => write!(f, "Invalid worker address: {}", err),
            Self::InvalidJsWorkerPath(err) => write!(
                f,
                "Invalid js worker path: {}. Make sure file at path exists and path is valid.",
                err
            ),
            Self::InvalidGlobalJsRendererPath(err) => write!(
                f,
                "Invalid global js renderer path: {}. Make sure file at path exists and path is valid.",
                err
            ),
            Self::SpawnNodeProcessError(err) => {
                write!(f, "Failed to spawn worker process: {}", err)
            }
        }
    }
}

#[derive(Debug)]
pub enum RenderingError {
    WorkerIsUnavailable,
    ConnectionError(io::Error),
    InvalidUri,
    GlobalRendererNotProvided,
    UrlSerializationError(serde_json::Error),
    DataSerializationError(serde_json::Error),
    RenderRequestError(io::Error),
    RenderResponseError(io::Error),
    JsExceptionDuringRendering(String),
}

impl fmt::Display for RenderingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkerIsUnavailable => write!(f, "Worker is unavailable"),
            Self::ConnectionError(err) => write!(f, "Connection error: {}", err),
            Self::InvalidUri => write!(f, "Invalid URI"),
            Self::GlobalRendererNotProvided => write!(f, "Rendering request supposed to use global js renderer but it wasn't provided on renderer initialization"),
            Self::UrlSerializationError(err) => write!(f, "Failed to serialize URL: {}", err),
            Self::DataSerializationError(err) => write!(f, "Failed to serialize data: {}", err),
            Self::RenderRequestError(err) | Self::RenderResponseError(err) => {
                write!(f, "Failed to communicate with rendering process: {}", err)
            }
            Self::JsExceptionDuringRendering(err) => {
                write!(f, "JS Exception during rendering: {}", err)
            }
        }
    }
}
