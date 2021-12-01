use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Trace {
    trace: Vec<Vec<Option<usize>>>,
}
