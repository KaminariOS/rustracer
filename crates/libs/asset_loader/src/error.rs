use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Failed to load gltf file: {0}")]
    Load(String),
    #[error("Unsupported gltf feature: {0}")]
    Support(String),
}
