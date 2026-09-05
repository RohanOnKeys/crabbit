use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrabbitError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("segment {0} not found")]
    SegmentNotFound(u32),
    #[error("corrupt segment: {0}")]
    CorruptSegment(String),
}
