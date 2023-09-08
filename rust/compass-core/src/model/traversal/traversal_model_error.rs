use super::state::traversal_state::TraversalState;

#[derive(thiserror::Error, Debug, Clone)]
pub enum TraversalModelError {
    #[error("failure building traversal model from file {0}: {1}")]
    FileReadError(String, String),
    #[error("failure building traversal model: {0}")]
    BuildError(String),
    #[error("numeric error during calculation: {0}")]
    NumericError(String),
    #[error("index {0} for {1} not found on search state {2:?}")]
    StateVectorIndexOutOfBounds(usize, String, TraversalState),
    #[error("id {0} for id type {1} not found in tabular edge cost function {2}")]
    MissingIdInTabularCostFunction(String, String, String),
    #[error("internal error: {0}")]
    InternalError(String),
    #[error("prediction model failed with error {0}")]
    PredictionModel(String),
}