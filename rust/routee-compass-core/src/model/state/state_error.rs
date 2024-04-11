use crate::model::traversal::state::state_variable::StateVar;

#[derive(thiserror::Error, Debug)]
pub enum StateError {
    #[error("attempting to encode {0} as state variable when value is a {1}")]
    EncodeError(String, String),
    #[error("attempting to decode {0} as a {1} when codec expects a {2}")]
    DecodeError(StateVar, String, String),
    #[error("value {0} is not a valid {1}")]
    ValueError(StateVar, String),
    #[error("unknown state variable name {0}, should be one of {1}")]
    UnknownStateVariableName(String, String),
    #[error("invalid state variable index {0}, should be in range [0, {1})")]
    InvalidStateVariableIndex(usize, usize),
    #[error("expected feature to have type '{0}' but found '{1}'")]
    UnexpectedFeatureType(String, String),
    #[error("expected feature unit to be {0} but found {1}")]
    UnexpectedFeatureUnit(String, String),
    #[error("{0}")]
    BuildError(String),
    #[error("{0}")]
    RuntimeError(String),
}
