use thiserror::Error; // For colouring log messages.

#[derive(Debug, Error)]
pub enum Error {
    #[error("{}", .0)]
    Glob(#[from] glob::GlobError),
    #[error("{}", .0)]
    GlobPattern(#[from] glob::PatternError),
    #[error("No valid meshes were found")]
    NoMeshesFound,
    #[error("Configuration load error: {}", .0)]
    ConfigLoad(#[from] std::io::Error),
    #[error("Only JSON and RON configuration formats are supported. Unknown config extension: {}", .0)]
    ConfigUnsupported(String),
    #[error("Configuration RON deserialization error: {}", .0)]
    ConfigDeserializeRON(#[from] ron::error::SpannedError),
    #[error("Configuration JSON deserialization error: {}", .0)]
    ConfigDeserializeJSON(#[from] serde_json::error::Error),
    #[error("Configuration RON serialization error: {}", .0)]
    ConfigSerializeRON(#[from] ron::error::Error),
}
