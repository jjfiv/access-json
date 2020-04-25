use crate::query::{JSONQuery, QueryElement};
use crate::AnySerializable;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum State {
    StartMap,
    MapKey,
    MapKeyStr(String),
    MapValue,
    StructField,
    StructValue,
    /// Keep track of where we are, index of length:
    Sequence(usize, usize),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryExecutor {
    query: Vec<QueryElement>,
    current_path: Vec<QueryElement>,
    state: Vec<State>,
    result: Option<serde_json::Value>,
}
impl QueryExecutor {
    pub fn new(query: &JSONQuery) -> Self {
        Self {
            query: query.elements.clone(),
            current_path: Vec::new(),
            state: Vec::new(),
            result: None,
        }
    }
    fn found_match(&self) -> bool {
        println!("found_match: {:?}", self.current_path);
        return self.query == self.current_path;
    }
    fn set_result(&mut self, found: &dyn AnySerializable) -> Result<(), QueryExecError> {
        if self.result.is_some() {
            Err(QueryExecError::TwoMatchingPaths)
        } else {
            let value = serde_json::to_value(found)?;
            println!("set_result {:?}", value);
            self.result = Some(value);
            Ok(())
        }
    }
    pub fn get_result(self) -> Option<serde_json::Value> {
        self.result
    }

    fn enter_name(&mut self, name: &str) -> bool {
        self.current_path.push(QueryElement::field(name));
        // for now, just always true
        // TODO: only enter structs that are on our query's path!
        true
    }
    fn must_enter_name(&mut self, name: &str) {
        self.current_path.push(QueryElement::field(name));
    }
    fn exit_name(&mut self, name: &str) {
        let top = self.current_path.pop();
        assert_eq!(Some(QueryElement::field(name)), top);
    }
    fn exit_unknown_name(&mut self) -> Result<(), QueryExecError> {
        match self.current_path.pop() {
            Some(QueryElement::AccessField { .. }) => Ok(()),
            e => Err(QueryExecError::InternalError(format!(
                "Expected Name: {:?}",
                e
            ))),
        }
    }
    fn enter_sequence(&mut self, length: Option<usize>) {
        self.state.push(State::Sequence(
            0,
            length.expect("All sequences have lengths?"),
        ));
    }
    fn sequence_element<T: ?Sized>(&mut self, value: &T) -> Result<(), QueryExecError>
    where
        T: serde::ser::Serialize,
    {
        let index = match self.state.pop() {
            Some(State::Sequence(idx, len)) => {
                assert!(idx < len);
                self.state.push(State::Sequence(idx + 1, len));
                idx
            }
            _ => panic!("state should be sequence"),
        };
        if self.enter_index(index) {
            let output = value.serialize(&mut *self);
            self.exit_index(index);
            output
        } else {
            Ok(())
        }
    }
    fn enter_index(&mut self, index: usize) -> bool {
        self.current_path.push(QueryElement::array_item(index));
        // for now, just always true
        // TODO: only enter indices that are on our query's path!
        true
    }
    fn exit_index(&mut self, index: usize) {
        let top = self.current_path.pop();
        assert_eq!(Some(QueryElement::array_item(index)), top);
    }
    fn exit_sequence(&mut self) -> Result<(), QueryExecError> {
        let top = self.state.pop();
        match top {
            Some(State::Sequence(pos, len)) => {
                assert_eq!(pos, len);
                Ok(())
            }
            found => Err(QueryExecError::InternalError(format!(
                "Bad exit_sequence state: {:?}",
                found
            ))),
        }
    }
    fn enter_map(&mut self) {
        self.state.push(State::StartMap);
    }
    fn exit_map(&mut self) {
        let top = self.state.pop();
        assert_eq!(top, Some(State::StartMap));
    }
    fn enter_map_key(&mut self) {
        self.state.push(State::MapKey);
    }
    fn exit_map_key(&mut self) -> Result<(), QueryExecError> {
        // Leave MapKeyStr on state stack!
        match self.state.last() {
            Some(State::MapKeyStr(_)) => Ok(()),
            _ => Err(QueryExecError::InternalError(format!(
                "Map key not a simple String! {:?}",
                self.current_path
            ))),
        }
    }
    fn enter_map_value(&mut self) {
        match self.state.last() {
            Some(State::MapKeyStr(_)) => {}
            _ => panic!(
                "enter_map_value {:?} state={:?}",
                self.current_path, self.state
            ),
        };
        self.state.push(State::MapValue);
    }
    fn exit_map_value(&mut self) -> Result<(), QueryExecError> {
        match self.state.pop() {
            Some(State::MapValue) => {}
            actual => Err(QueryExecError::InternalError(format!(
                "Expected MapValue state, found: {:?}",
                actual
            )))?,
        }
        match self.state.pop() {
            Some(State::MapKeyStr(name)) => {
                self.exit_name(&name);
            }
            actual => Err(QueryExecError::InternalError(format!(
                "Expected MapKeyStr state, found: {:?}",
                actual
            )))?,
        }
        match self.state.pop() {
            Some(State::MapKey) => Ok(()),
            actual => Err(QueryExecError::InternalError(format!(
                "Expected MapKey state, found: {:?}",
                actual
            )))?,
        }
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
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&v)?;
        }
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        println!(
            "serialize_str {} @ {:?}, state={:?}",
            v, self.current_path, self.state
        );
        match self.state.last() {
            Some(State::MapKey) => {
                self.state.push(State::MapKeyStr(v.to_string()));
                self.enter_name(v);
                Ok(())
            }
            Some(State::MapKeyStr(_)) | Some(_) => {
                if self.found_match() {
                    self.set_result(&v.to_string())?;
                }
                Ok(())
            }
            Option::None => Err(QueryExecError::InternalError(
                "&str value with no state!".into(),
            )),
        }
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        if self.found_match() {
            self.set_result(&serde_json::Value::Null)?;
        }
        Ok(())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        if !self.enter_name(name) {
            return Ok(());
        }
        let output = self.serialize_unit();
        self.exit_name(name);
        output
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        if !self.enter_name(name) {
            return Ok(());
        }
        let output = self.serialize_str(variant);
        self.exit_name(name);
        output
    }
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        // TODO test ignoring names of newtype structs; e.g.,
        // struct Meters(f64) is serialized as jus ta f64 and we don't care about the name of that type...?
        value.serialize(&mut *self)
    }
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        if !self.enter_name(variant) {
            return Ok(());
        }
        let output = value.serialize(&mut *self);
        self.exit_name(variant);
        output
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.enter_sequence(len);
        Ok(self)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        // TODO: what's a tuple-struct name in JSON output -- should be nothing?
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.must_enter_name(variant);
        Ok(self)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.enter_map();
        Ok(self)
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.must_enter_name(variant);
        Ok(self)
    }
}

impl<'a> serde::ser::SerializeSeq for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.sequence_element(value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_sequence()
    }
}

impl<'a> serde::ser::SerializeMap for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        // TODO not sure how to check this is a path we want.
        // Serde does not enforce string-only keys, but JSON does.
        // So we have a &T here and not a &str or &String like we'd want for checking.
        self.enter_map_key();
        key.serialize(&mut **self)?;
        self.exit_map_key()
    }
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.enter_map_value();
        value.serialize(&mut **self)?;
        self.exit_map_value()
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_map();
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTuple for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.sequence_element(value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_sequence()
    }
}
impl<'a> serde::ser::SerializeTupleStruct for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.sequence_element(value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_sequence()
    }
}
impl<'a> serde::ser::SerializeTupleVariant for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.sequence_element(value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_sequence().and_then(|_| self.exit_unknown_name())
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
        if self.enter_name(key) {
            value.serialize(&mut **self)?;
            self.exit_name(key);
        }
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_unknown_name()
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
        if self.enter_name(key) {
            value.serialize(&mut **self)?;
            self.exit_name(key);
        }
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_unknown_name()
    }
}
