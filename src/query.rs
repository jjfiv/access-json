use crate::query_executor::{QueryExecError, QueryExecutor};
use crate::query_parser::{parse_query, QueryParseErr};
use crate::AnySerializable;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct LinearResult {
    /// The relative path to this result from the query.
    pub path: Vec<QueryElement>,
    /// The value element at this path.
    pub result: serde_json::Value,
}

impl LinearResult {
    pub(crate) fn new(path: Vec<QueryElement>, result: serde_json::Value) -> Self {
        Self { path, result }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for QueryElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryElement::Field(name) => write!(f, ".{}", name),
            QueryElement::ArrayItem(index) => write!(f, "[{}]", index),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct JSONQuery {
    pub elements: Vec<QueryElement>,
}
impl std::fmt::Display for JSONQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for elem in self.elements.iter() {
            write!(f, "{}", elem)?
        }
        Ok(())
    }
}

impl JSONQuery {
    pub fn new(elements: Vec<QueryElement>) -> Self {
        Self { elements }
    }
    pub fn parse(input: &str) -> Result<Self, QueryParseErr> {
        Ok(Self {
            elements: parse_query(input)?,
        })
    }
    #[cfg(test)]
    pub fn single(q: QueryElement) -> Self {
        Self::new(vec![q])
    }
    pub fn search(
        &self,
        target: &dyn AnySerializable,
    ) -> Result<Vec<LinearResult>, QueryExecError> {
        let mut runner = QueryExecutor::new(self);
        let () = target.serialize(&mut runner)?;
        Ok(runner.get_results())
    }

    #[cfg(test)]
    pub fn execute_for_value(
        &self,
        target: &dyn AnySerializable,
    ) -> Result<Option<serde_json::Value>, QueryExecError> {
        let mut runner = QueryExecutor::new(self);
        let () = target.serialize(&mut runner)?;
        let results = runner.get_results();
        match results.len() {
            0 => Ok(None),
            1 => Ok(Some(results[0].result.clone())),
            _ => panic!("Multiple results for query: {}; {:?}", self, results),
        }
    }
}
