pub type FileBaseResult<T> = Result<T, FileBaseError>;

pub mod jobs;

#[derive(Debug, thiserror::Error)]
pub enum FileBaseError {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("queue error: {0}")]
    Queue(String),
}

impl From<redis::RedisError> for FileBaseError {
    fn from(err: redis::RedisError) -> Self {
        FileBaseError::Queue(err.to_string())
    }
}
