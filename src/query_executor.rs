use crate::query::{JSONQuery, QueryElement};
use crate::AnySerializable;
use std::cell::RefCell;

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryExecutor {
    query: Vec<QueryElement>,
    current_path: Vec<QueryElement>,
    result: Option<serde_json::Value>,
}
impl QueryExecutor {
    pub fn new(query: &JSONQuery) -> Self {
        Self {
            query: query.elements.clone(),
            current_path: Vec::new(),
            result: None,
        }
    }
    fn found_match(&self) -> bool {
        return self.query == self.current_path;
    }
    fn set_result(&mut self, found: &dyn AnySerializable) -> Result<(), QueryExecError> {
        if self.result.is_some() {
            Err(QueryExecError::TwoMatchingPaths)
        } else {
            self.result = Some(serde_json::to_value(found)?);
            Ok(())
        }
    }
    pub fn get_result(self) -> Option<serde_json::Value> {
        self.result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum QueryExecError {
    TwoMatchingPaths,
    InternalError(String),
    Serialization(String),
}

impl From<serde_json::Error> for QueryExecError {
    fn from(err: serde_json::Error) -> Self {
        Self::InternalError(format!("{:?}", err))
    }
}

impl std::fmt::Display for QueryExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)?;
        Ok(())
    }
}
impl std::error::Error for QueryExecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl serde::ser::Error for QueryExecError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        QueryExecError::Serialization(format!("{}", msg))
    }
}

impl<'a> serde::Serializer for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        todo!()
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!()
    }
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        todo!()
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}

impl<'a> serde::ser::SerializeSeq for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> serde::ser::SerializeMap for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a> serde::ser::SerializeTuple for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a> serde::ser::SerializeTupleStruct for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a> serde::ser::SerializeTupleVariant for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a> serde::ser::SerializeStruct for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> serde::ser::SerializeStructVariant for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
