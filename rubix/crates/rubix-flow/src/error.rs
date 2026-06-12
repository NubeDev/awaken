//! Board loading/build errors.

#[derive(Debug, thiserror::Error)]
pub enum FlowError {
    #[error("unknown board component `{0}`")]
    UnknownComponent(String),
    #[error("board build failed: {0}")]
    Build(String),
}
