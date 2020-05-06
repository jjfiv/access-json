//! # Query a subset of your Serde-Serializable Data
//!
//! This library allows you to execute a simple path-query against any serde-serializable object; returning the result as a ``Option<serde_json::Value>``.
//!
//! ```
//! use access_json::JSONQuery;
//! use std::collections::HashMap;
//! use serde_json;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut data: HashMap<&str, u32> = HashMap::default();
//! data.insert("cat", 9);
//!
//! let query = JSONQuery::parse(".cat")?; // QueryParseErr
//! let output = query.execute(&data)?; // QueryExecErr
//! let expected = serde_json::to_value(&9)?; // You must derive Serialize!
//!
//! assert_eq!(Some(expected), output);
//! # Ok(())
//! # }
//! ```
//!
//! ## A More Complex, Nested Example
//!
//!
//! ```
//! use access_json::JSONQuery;
//! use serde_json::{self, Value};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let data: Value = serde_json::from_str(r#"{
//!    "items": [
//!       {
//!          "unwanted": 7,
//!          "wanted": {"x": 3, "y": 7},
//!          "array": [3,2,1]
//!       },
//!       {
//!          "whatever": true
//!       }
//!    ]
//! }"#)?;
//!
//! // We can reference dictionary fields and array indices together:
//! let output = JSONQuery::parse(".items[1].whatever")?.execute(&data)?;
//! let expected = serde_json::to_value(&true)?;
//! assert_eq!(Some(expected), output);
//!
//! // We can have results be of any-size sub-tree, e.g., a whole array or vec.
//! let output = JSONQuery::parse(".items[0].array")?.execute(&data)?;
//! let expected = serde_json::to_value(&vec![3,2,1])?;
//! assert_eq!(Some(expected), output);
//! # Ok(())
//! # }
//! ```
//!
//! ## Just ``#[derive(Serialize)]`` to query any struct or enum:
//!
//! ```
//! use access_json::JSONQuery;
//! #[macro_use]
//! extern crate serde_derive;
//!
//! #[derive(Serialize)]
//! struct Dog {
//!    name: String,
//!    age: i32,
//!    favorites: Vec<String>,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let data = Dog {
//!     name: "Buddy".into(),
//!     age: 14,
//!     favorites: vec!["walks".into(), "naps".into()],
//! };
//!
//! let found = JSONQuery::parse(".name")?.execute(&data)?.unwrap();
//! assert_eq!("Buddy", found);
//! # Ok(())
//! # }
//! ```
//!

#[macro_use]
extern crate serde_derive;

pub use erased_serde::Serialize as AnySerializable;

pub mod query;
pub mod query_executor;
pub mod query_parser;

#[doc(inline)]
pub use query::JSONQuery;
#[doc(inline)]
pub use query_executor::QueryExecErr;
#[doc(inline)]
pub use query_parser::QueryParseErr;

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
        let world_q = JSONQuery::single(QueryElement::field("world"));
        let found = world_q.execute(&data).unwrap();
        assert_eq!(found, Some(JV::Number(5.into())));

        let hello_q = JSONQuery::single(QueryElement::field("hello"));
        let found = hello_q.execute(&data).unwrap();
        assert_eq!(found, Some(JV::Number(7.into())));
    }

    #[test]
    fn test_query_vec() {
        let data = vec![0, 1, 2, 3, 4, 5];

        for i in 0..data.len() {
            let elem_q = JSONQuery::single(QueryElement::array_item(i));
            let found = elem_q.execute(&data).unwrap().unwrap();
            assert_eq!(found, (JV::Number(i.into())));
        }

        let missing_q = JSONQuery::single(QueryElement::array_item(17));
        let found = missing_q.execute(&data).unwrap();
        assert_eq!(None, found);
    }

    #[test]
    fn test_tuple() {
        let point = (17, 39);

        let first_q = JSONQuery::single(QueryElement::array_item(0));
        let found = first_q.execute(&point).unwrap().unwrap();
        assert_eq!(found, (JV::Number(17.into())));

        let second_q = JSONQuery::single(QueryElement::array_item(1));
        let found = second_q.execute(&point).unwrap().unwrap();

        assert_eq!(found, (JV::Number(39.into())));
        let missing_q = JSONQuery::single(QueryElement::array_item(3));
        let found = missing_q.execute(&point).unwrap();
        assert_eq!(None, found);
    }

    #[derive(PartialEq, Eq, Clone, Serialize)]
    struct Example {
        name: String,
        age: i32,
        favorites: Vec<String>,
    }

    #[test]
    fn test_example_struct() {
        let data = Example {
            name: "Buddy".into(),
            age: 14,
            favorites: vec!["walks".into(), "naps".into()],
        };

        let name_q = JSONQuery::single(QueryElement::field("name"));
        assert_eq!("Buddy", name_q.execute(&data).unwrap().unwrap());
        let age_q = JSONQuery::single(QueryElement::field("age"));
        assert_eq!(14, age_q.execute(&data).unwrap().unwrap());

        let first_favorite_q = JSONQuery::new(vec![
            QueryElement::field("favorites"),
            QueryElement::array_item(0),
        ]);
        assert_eq!("walks", first_favorite_q.execute(&data).unwrap().unwrap());
    }

    #[test]
    fn test_whole_object_results() {
        let data = Example {
            name: "Buddy".into(),
            age: 14,
            favorites: vec!["walks".into(), "naps".into()],
        };

        let all_favorites = JSONQuery::single(QueryElement::field("favorites"));
        let expected: Vec<JV> = vec!["walks".into(), "naps".into()];
        assert_eq!(
            Some(&expected),
            all_favorites.execute(&data).unwrap().unwrap().as_array()
        );
    }

    #[derive(Serialize)]
    struct NestedStructs {
        dog: Example,
        truthiness: bool,
        score: i32,
    }

    #[test]
    fn test_nested_structs() {
        let data = NestedStructs {
            dog: Example {
                name: "Buddy".into(),
                age: 14,
                favorites: vec!["walks".into(), "naps".into()],
            },
            truthiness: false,
            score: -77,
        };

        assert_eq!(
            JSONQuery::parse(".dog.name")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap(),
            "Buddy"
        );
        assert_eq!(
            JSONQuery::parse(".truthiness")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap(),
            false
        )
    }

    #[test]
    fn test_vec_structs() {
        let data: Vec<Example> = vec![
            Example {
                name: "Buddy".into(),
                age: 14,
                favorites: vec![],
            },
            Example {
                name: "Tuukka".into(),
                age: 6,
                favorites: vec![],
            },
        ];
        assert_eq!(
            JSONQuery::parse("[0].name")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap(),
            "Buddy"
        );
        assert_eq!(
            JSONQuery::parse("[1].name")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap(),
            "Tuukka"
        )
    }

    #[derive(Serialize)]
    enum Pet {
        Bird,
        Dog(Example),
        Cat { lives: u32 },
        Digits(u32, u32, u32),
    }

    #[test]
    fn test_enum_examples() {
        let buddy = Example {
            name: "Buddy".into(),
            age: 14,
            favorites: vec!["walks".into(), "naps".into()],
        };
        let data = vec![
            Pet::Bird,
            Pet::Dog(buddy.clone()),
            Pet::Cat { lives: 9 },
            Pet::Digits(7, 5, 6),
        ];
        // For debugging:
        //println!("json: {}", serde_json::to_string_pretty(&data).unwrap());

        assert_eq!(
            "Bird",
            JSONQuery::parse("[0]")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            14,
            JSONQuery::parse("[1].Dog.age")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            serde_json::to_value(buddy).unwrap(),
            JSONQuery::parse("[1].Dog")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            "Bird",
            JSONQuery::parse("[0]")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            9,
            JSONQuery::parse("[2].Cat.lives")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            serde_json::to_value(vec![7, 5, 6]).unwrap(),
            JSONQuery::parse("[3].Digits")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
    }

    // tuple-struct
    #[derive(Serialize)]
    struct Point(u32, u32);

    #[test]
    fn test_tuple_struct() {
        let data = vec![Point(1, 2), Point(3, 4)];
        assert_eq!(
            serde_json::to_value(vec![1, 2]).unwrap(),
            JSONQuery::parse("[0]")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            4,
            JSONQuery::parse("[1][1]")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
    }

    #[test]
    fn test_list_of_lists() {
        let data = vec![
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![vec![7, 8, 9], vec![10, 11, 12]],
        ];

        assert_eq!(
            serde_json::to_value(&vec![vec![1, 2, 3], vec![4, 5, 6]]).unwrap(),
            JSONQuery::parse("[0]")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        )
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    struct UnitStruct;

    #[test]
    fn test_unit_struct() {
        let nothings = vec![UnitStruct, UnitStruct];
        assert_eq!(
            serde_json::to_value(&UnitStruct).unwrap(),
            JSONQuery::parse("[0]")
                .unwrap()
                .execute(&nothings)
                .unwrap()
                .unwrap()
        )
    }

    #[derive(Debug, PartialEq, Eq, Serialize)]
    struct NewType(i32);

    #[test]
    fn test_newtype_struct() {
        let data = vec![NewType(-2), NewType(3)];
        assert_eq!(
            -2,
            JSONQuery::parse("[0]")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            3,
            JSONQuery::parse("[1]")
                .unwrap()
                .execute(&data)
                .unwrap()
                .unwrap()
        )
    }
}
