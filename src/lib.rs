#[macro_use]
extern crate serde_derive;

pub use erased_serde::Serialize as AnySerializable;

pub mod query;
pub mod query_executor;

#[cfg(test)]
mod tests {
    use super::query::*;
    use serde_json::Value as JV;
    use std::collections::HashMap;

    #[test]
    fn test_jq_simple() {
        let mut data: HashMap<&str, usize> = HashMap::default();
        data.insert("hello", 7);
        data.insert("world", 5);
        let world_q = JSONQuery::new(vec![QueryElement::field("world")]);
        let found = world_q.execute(&data).expect("No serialization errors.");
        assert_eq!(found, Some(JV::Number(7.into())));
    }
}
