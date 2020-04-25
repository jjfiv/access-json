use crate::query_executor::{QueryExecError, QueryExecutor};
use crate::AnySerializable;
use serde::Serialize;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum QueryElement {
    Field(String),
    ArrayItem(usize),
}

impl QueryElement {
    pub fn field(field: &str) -> Self {
        Self::Field(field.into())
    }
    pub fn array_item(index: usize) -> Self {
        Self::ArrayItem(index)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct JSONQuery {
    pub elements: Vec<QueryElement>,
}

impl JSONQuery {
    pub fn new(elements: Vec<QueryElement>) -> Self {
        Self { elements }
    }
    pub fn single(q: QueryElement) -> Self {
        Self::new(vec![q])
    }
    pub fn execute(
        &self,
        target: &dyn AnySerializable,
    ) -> Result<Option<serde_json::Value>, QueryExecError> {
        let mut runner = QueryExecutor::new(self);
        match target.serialize(&mut runner) {
            Ok(()) | Err(QueryExecError::EarlyReturnHack) => Ok(runner.get_result()),
            Err(e) => Err(e),
        }
    }
}
