use crate::query_executor::{QueryExecErr, QueryExecutor};
use crate::query_parser::{parse_query, QueryParseErr};
use crate::AnySerializable;
use serde::Serialize;

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

/// This is the main interface to this library.
/// Create a new JSONQuery by calling parse.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct JSONQuery {
    /// A list of access-elements, field names or array indices.
    pub elements: Vec<QueryElement>,
}

/// This is a way to visualize a JSONQuery object as a parse-able string.
impl std::fmt::Display for JSONQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for elem in self.elements.iter() {
            write!(f, "{}", elem)?
        }
        Ok(())
    }
}

impl JSONQuery {
    /// Construct a new JSONQuery object from discrete elements.
    pub(crate) fn new(elements: Vec<QueryElement>) -> Self {
        Self { elements }
    }

    /// Construct a new JSONQuery object from an example string.
    ///
    /// ```
    /// use access_json::JSONQuery;
    /// use access_json::query::QueryElement; // Only needed to validate our parsing.
    ///
    /// assert_eq!(
    ///   JSONQuery::parse(".field.array[8]").unwrap().elements,
    ///   vec![QueryElement::field("field"),
    ///        QueryElement::field("array"),
    ///        QueryElement::array_item(8)]);
    /// ```
    pub fn parse(input: &str) -> Result<Self, QueryParseErr> {
        Ok(Self::new(parse_query(input)?))
    }

    #[cfg(test)]
    pub fn single(q: QueryElement) -> Self {
        Self::new(vec![q])
    }

    /// Execute a JSONQuery object against any serde-serializable object.
    ///
    /// ```
    /// use access_json::JSONQuery;
    /// use std::collections::HashMap;
    /// use serde_json;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut data: HashMap<&str, u32> = HashMap::default();
    /// data.insert("cat", 9);
    ///
    /// let query = JSONQuery::parse(".cat")?;
    /// let output = query.execute(&data)?;
    /// let expected = serde_json::to_value(&9)?;
    ///
    /// assert_eq!(Some(expected), output);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute(
        &self,
        target: &dyn AnySerializable,
    ) -> Result<Option<serde_json::Value>, QueryExecErr> {
        let mut runner = QueryExecutor::new(self)?;
        target.serialize(&mut runner)?;
        Ok(runner.get_result())
    }
}
