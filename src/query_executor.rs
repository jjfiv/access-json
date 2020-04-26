use crate::query::{JSONQuery, LinearResult, QueryElement};
use crate::AnySerializable;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum State {
    /// When we start serializing a Map element.
    StartMap,
    /// When we have encountered a Map key but still need to serialize it.
    MapKey,
    /// When we have encountered a Str in MapKey state.
    MapKeyStr(String),
    /// When we have the name of the field and begin serializing/visiting the MapValue.
    MapValue,
    /// Keep track of where we are, index of length:
    Sequence(usize, usize),
}

enum NextStep<'a> {
    NotMatching,
    Found(&'a QueryElement),
    IsMatch(&'a [QueryElement]),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryExecutor {
    query: Vec<QueryElement>,
    current_path: Vec<QueryElement>,
    state: Vec<State>,
    results: Vec<LinearResult>,
}
impl QueryExecutor {
    pub fn new(query: &JSONQuery) -> Self {
        Self {
            query: query.elements.clone(),
            current_path: Vec::new(),
            state: Vec::new(),
            results: Vec::new(),
        }
    }
    fn next_step(&self) -> NextStep<'_> {
        let mut i = 0;
        while i < self.query.len() && i < self.current_path.len() {
            if self.query[i] != self.current_path[i] {
                return NextStep::NotMatching;
            }
            i += 1;
        }
        // we have matched until one of us exhausted (query) or current_path.
        if self.current_path.len() < self.query.len() {
            NextStep::Found(&self.query[i])
        } else {
            NextStep::IsMatch(&self.current_path[i..])
        }
    }
    /// Find the relative path to our current location, but only if we're matching the query.
    fn relative_path(&self) -> Option<Vec<QueryElement>> {
        match self.next_step() {
            NextStep::IsMatch(relative) => Some(relative.to_vec()),
            _ => None,
        }
    }
    fn possible_result(&mut self, found: &dyn AnySerializable) -> Result<(), QueryExecError> {
        if let Some(relative) = self.relative_path() {
            self.results
                .push(LinearResult::new(relative, serde_json::to_value(found)?))
        }
        Ok(())
    }
    pub fn get_results(self) -> Vec<LinearResult> {
        self.results
    }

    /// When we have recursive control over entering a scope or not, only enter if it advances our query match!
    fn enter_name(&mut self, name: &str) -> bool {
        let continues_match = match self.next_step() {
            NextStep::IsMatch(_) => true,
            NextStep::Found(QueryElement::Field(field)) => name == field,
            _ => false,
        };
        if continues_match {
            self.current_path.push(QueryElement::field(name));
        }
        continues_match
    }

    /// Sometimes we do not have control over entering a scope; so we just push without checking whether it advances our match or not.
    fn must_enter_name(&mut self, name: &str) {
        self.current_path.push(QueryElement::field(name));
    }
    fn exit_name(&mut self, name: &str) {
        let top = self.current_path.pop();
        assert_eq!(Some(QueryElement::field(name)), top);
    }
    fn exit_unknown_name(&mut self) -> Result<(), QueryExecError> {
        match self.current_path.pop() {
            Some(QueryElement::Field(_what)) => Ok(()),
            e => Err(QueryExecError::InternalError(format!(
                "Expected Name, but found {:?}; state={:?}",
                e, self.state
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
            x => panic!(
                "state should be sequence but was {:?}; path={:?}",
                x, self.current_path
            ),
        };
        if self.enter_index(index) {
            value.serialize(&mut *self)?;
            self.exit_index(index);
        }
        Ok(())
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
            actual => {
                return Err(QueryExecError::InternalError(format!(
                    "Expected MapValue state, found: {:?}",
                    actual
                )))
            }
        }
        match self.state.pop() {
            Some(State::MapKeyStr(name)) => {
                self.exit_name(&name);
            }
            actual => {
                return Err(QueryExecError::InternalError(format!(
                    "Expected MapKeyStr state, found: {:?}",
                    actual
                )))
            }
        }
        match self.state.pop() {
            Some(State::MapKey) => Ok(()),
            actual => Err(QueryExecError::InternalError(format!(
                "Expected MapKey state, found: {:?}",
                actual
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum QueryExecError {
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
        // Note: erased_serde brings us here on error.
        QueryExecError::Serialization(msg.to_string())
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
        self.possible_result(&v)
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.possible_result(&v)
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        match self.state.last() {
            Some(State::MapKey) => {
                self.state.push(State::MapKeyStr(v.to_string()));
                self.must_enter_name(v);
                Ok(())
            }
            Some(State::MapKeyStr(_)) => Err(QueryExecError::InternalError(
                "Shouldn't see multiple str for the same key!".into(),
            )),
            Some(_) => self.possible_result(&v),
            Option::None => Err(QueryExecError::InternalError(
                "&str value with no state!".into(),
            )),
        }
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
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
        self.possible_result(&serde_json::Value::Null)
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        if self.enter_name(name) {
            self.serialize_unit()?;
            self.exit_name(name);
        }
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        if self.enter_name(name) {
            self.serialize_str(variant)?;
            self.exit_name(name);
        }
        Ok(())
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
        if self.enter_name(variant) {
            value.serialize(&mut *self)?;
            self.exit_name(variant);
        }
        Ok(())
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
        self.exit_map();
        Ok(())
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
