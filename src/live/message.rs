use std::fmt::{Display, Formatter};
use serde_json::Value;

#[derive(Debug)]
pub struct LiveMessage {
    pub data: Value
}

impl LiveMessage {
    pub fn new(data: Value) -> Self {
        Self { data }
    }
}

impl Display for LiveMessage {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.data)
    }
}
