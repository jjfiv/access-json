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
    fn test_query_hashmap() {
        let mut data: HashMap<&str, usize> = HashMap::default();
        data.insert("hello", 7);
        data.insert("world", 5);
        let world_q = JSONQuery::new(vec![QueryElement::field("world")]);
        let found = world_q.execute(&data).unwrap();
        assert_eq!(found, Some(JV::Number(5.into())));

        let hello_q = JSONQuery::new(vec![QueryElement::field("hello")]);
        let found = hello_q.execute(&data).unwrap();
        assert_eq!(found, Some(JV::Number(7.into())));
    }

    #[test]
    fn test_query_vec() {
        let data = vec![0, 1, 2, 3, 4, 5];

        for i in 0..data.len() {
            let elem_q = JSONQuery::new(vec![QueryElement::array_item(i)]);
            let found = elem_q.execute(&data).unwrap().unwrap();
            assert_eq!(found, (JV::Number(i.into())));
        }

        let missing_q = JSONQuery::new(vec![QueryElement::array_item(17)]);
        let found = missing_q.execute(&data).unwrap();
        assert_eq!(None, found);
    }

    #[test]
    fn test_tuple() {
        let point = (17, 39);

        let first_q = JSONQuery::new(vec![QueryElement::array_item(0)]);
        let found = first_q.execute(&point).unwrap().unwrap();
        assert_eq!(found, (JV::Number(17.into())));

        let second_q = JSONQuery::new(vec![QueryElement::array_item(1)]);
        let found = second_q.execute(&point).unwrap().unwrap();

        assert_eq!(found, (JV::Number(39.into())));
        let missing_q = JSONQuery::new(vec![QueryElement::array_item(3)]);
        let found = missing_q.execute(&point).unwrap();
        assert_eq!(None, found);
    }
}
