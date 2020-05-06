# access-json ![Rust](https://github.com/jjfiv/access-json/workflows/Rust/badge.svg)

Hijack serde to query large nested structures in Rust. For low-effort, read-only FFI.

## Query a subset of your Serde-Serializable Data

This library allows you to execute a simple path-query against any serde-serializable object; returning the result as a ``Option<serde_json::Value>``.

```rust
use access_json::JSONQuery;
use std::collections::HashMap;
use serde_json;

let mut data: HashMap<&str, u32> = HashMap::default();
data.insert("cat", 9);

let query = JSONQuery::parse(".cat")?; // QueryParseErr
let output = query.execute(&data)?; // QueryExecErr
let expected = serde_json::to_value(&9)?; // You must derive Serialize!

assert_eq!(Some(expected), output);
```

 ## A More Complex, Nested Example


 ```rust
 use access_json::JSONQuery;
 use serde_json::{self, Value};

 let data: Value = serde_json::from_str(r#"{
    "items": [
       {
          "unwanted": 7,
          "wanted": {"x": 3, "y": 7},
          "array": [3,2,1]
       },
       {
          "whatever": true
       }
    ]
 }"#)?;

 // We can reference dictionary fields and array indices together:
 let output = JSONQuery::parse(".items[1].whatever")?.execute(&data)?;
 let expected = serde_json::to_value(&true)?;
 assert_eq!(Some(expected), output);

 // We can have results be of any-size sub-tree, e.g., a whole array or vec.
 let output = JSONQuery::parse(".items[0].array")?.execute(&data)?;
 let expected = serde_json::to_value(&vec![3,2,1])?;
 assert_eq!(Some(expected), output);
 ```

 ## Just ``#[derive(Serialize)]`` to query any struct or enum:

 ```rust
 use access_json::JSONQuery;
 #[macro_use]
 extern crate serde_derive;

 #[derive(Serialize)]
 struct Dog {
    name: String,
    age: i32,
    favorites: Vec<String>,
 }

 let data = Dog {
     name: "Buddy".into(),
     age: 14,
     favorites: vec!["walks".into(), "naps".into()],
 };

 let found = JSONQuery::parse(".name")?.execute(&data)?.unwrap();
 assert_eq!("Buddy", found);
 ```
