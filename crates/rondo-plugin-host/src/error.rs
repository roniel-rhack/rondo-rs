use thiserror::Error;

#[derive(Error, Debug)]
pub enum HostError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("extism: {0}")]
    Extism(String),
    #[error("manifest mismatch: declared id `{declared}` does not match dir name `{dirname}`")]
    Mismatch { declared: String, dirname: String },
    #[error("unsupported api version `{0}`")]
    UnsupportedApi(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
