use crate::model::traversal::state::state_variable::StateVar;

/// describes an arbitrary state update operation.
pub type GenericStateUpdateOp = Box<dyn Fn(&StateVar, &StateVar) -> StateVar>;

/// represents the type of arithmetic operation used to update a state variable.
/// the specific index of a state variable is hidden via the StateModel, which
/// makes life harder, but protects against all sorts of indexing errors.
///
/// the StateModel exposes these operations through it's interface.
pub enum UpdateOperation {
    Replace,
    Add,
    Multiply,
    Max,
    Min,
    Function(GenericStateUpdateOp),
}

impl UpdateOperation {
    pub fn perform_operation(&self, prev: &StateVar, next: &StateVar) -> StateVar {
        match self {
            UpdateOperation::Replace => *next,
            UpdateOperation::Add => *prev + *next,
            UpdateOperation::Multiply => StateVar(prev.0 * next.0),
            UpdateOperation::Max => StateVar(prev.0.max(next.0)),
            UpdateOperation::Min => StateVar(prev.0.min(next.0)),
            UpdateOperation::Function(f) => f(prev, next),
        }
    }
}
