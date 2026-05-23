pub type FileBaseResult<T> = Result<T, FileBaseError>;

#[derive(Debug, thiserror::Error)]
pub enum FileBaseError {
    #[error("configuration error: {0}")]
    Configuration(String),
}
