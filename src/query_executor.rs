use crate::query::{JSONQuery, QueryElement};
use crate::AnySerializable;
use serde_json::Value as JSON;

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

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
enum ElementKind {
    Root,
    List,
    Map,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputStackFrame {
    kind: ElementKind,
    list_items: Vec<JSON>,
    map_keys: Vec<String>,
    map_values: Vec<JSON>,
}

impl Default for OutputStackFrame {
    fn default() -> Self {
        Self {
            kind: ElementKind::Root,
            list_items: Vec::new(),
            map_keys: Vec::new(),
            map_values: Vec::new(),
        }
    }
}
impl OutputStackFrame {
    fn list() -> OutputStackFrame {
        Self {
            kind: ElementKind::List,
            ..Default::default()
        }
    }
    fn map() -> OutputStackFrame {
        Self {
            kind: ElementKind::Map,
            ..Default::default()
        }
    }
    fn push_item(&mut self, item: JSON) {
        debug_assert_ne!(self.kind, ElementKind::Map);
        self.kind = ElementKind::List;
        self.list_items.push(item);
    }
    fn push_key(&mut self, item: String) {
        debug_assert_ne!(self.kind, ElementKind::List);
        self.kind = ElementKind::Map;
        self.map_keys.push(item);
    }
    fn push_value(&mut self, item: JSON) {
        debug_assert_ne!(self.kind, ElementKind::List);
        self.kind = ElementKind::Map;
        self.map_values.push(item);
    }
    /// When we've wrapped the level above us, hope we know what type of thing we are!
    fn push(&mut self, complex: OutputStackFrame) {
        let value = complex.finish();
        match self.kind {
            ElementKind::Root => self.push_item(value),
            ElementKind::List => self.push_item(value),
            ElementKind::Map => self.push_value(value),
        }
    }
    fn finish(self) -> JSON {
        match self.kind {
            ElementKind::Root => panic!("What's a ROOT? {:?}", self),
            ElementKind::List => JSON::Array(self.list_items),
            ElementKind::Map => {
                debug_assert_eq!(self.map_values.len(), self.map_values.len());
                let dict: serde_json::Map<String, JSON> = self
                    .map_keys
                    .into_iter()
                    .zip(self.map_values.into_iter())
                    .collect();
                JSON::Object(dict)
            }
        }
    }
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
    output: Vec<OutputStackFrame>,
}
impl QueryExecutor {
    pub fn new(query: &JSONQuery) -> Result<Self, QueryExecErr> {
        Ok(Self {
            query: query.elements.clone(),
            current_path: Vec::new(),
            state: Vec::new(),
            // Keep a list on the bottom of the stack for single-value answers.
            output: vec![Default::default()],
        })
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
    fn is_match(&self) -> bool {
        self.relative_path().is_some()
    }
    fn possible_result(&mut self, found: &dyn AnySerializable) -> Result<(), QueryExecErr> {
        if self.is_match() {
            let output_frame = self.output.last_mut().unwrap();
            match self.state.last().unwrap() {
                State::MapKey | State::MapKeyStr(_) => panic!(
                    "Shouldn't call possible_result here! {:?}, {:?}",
                    self.state, self.current_path
                ),
                // StartMap is the state in which we visit struct fields.
                State::StartMap | State::MapValue => {
                    output_frame.push_value(serde_json::to_value(found)?)
                }
                State::Sequence(_, _) => output_frame.push_item(serde_json::to_value(found)?),
            };
        }
        Ok(())
    }
    pub fn get_result(self) -> Option<JSON> {
        debug_assert_eq!(self.output.len(), 1);
        let output = &self.output[0];
        match output.kind {
            ElementKind::Root => output.list_items.get(0).cloned(),
            ElementKind::List => output.list_items.get(0).cloned(),
            ElementKind::Map => output.map_values.get(0).cloned(),
        }
    }

    /// When we have recursive control over entering a scope or not, only enter if it advances our query match!
    fn enter_name(&mut self, name: &str) -> bool {
        let continues_match = match self.next_step() {
            NextStep::IsMatch(_) => {
                // write this name to output.
                self.output.last_mut().unwrap().push_key(name.to_owned());
                true
            }
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
        if self.is_match() {
            // write this name to output.
            self.output.last_mut().unwrap().push_key(name.to_owned());
        }
    }
    fn exit_name(&mut self, name: Option<&str>) {
        let top = self.current_path.pop();
        if let Some(name) = name {
            debug_assert_eq!(Some(QueryElement::field(name)), top);
        }
    }
    fn enter_sequence(&mut self, length: Option<usize>) {
        if self.is_match() {
            self.output.push(OutputStackFrame::list());
        }
        self.state.push(State::Sequence(
            0,
            length.expect("All sequences have lengths?"),
        ));
    }
    fn sequence_element<T: ?Sized>(&mut self, value: &T) -> Result<(), QueryExecErr>
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
        let should_enter = match self.next_step() {
            NextStep::NotMatching => false,
            NextStep::Found(QueryElement::ArrayItem(x)) => (index == *x),
            NextStep::Found(_) => false,
            NextStep::IsMatch(_) => true,
        };
        if should_enter {
            self.current_path.push(QueryElement::array_item(index));
        }
        should_enter
    }
    fn exit_index(&mut self, index: usize) {
        let top = self.current_path.pop();
        debug_assert_eq!(Some(QueryElement::array_item(index)), top);
    }
    fn exit_sequence(&mut self) -> Result<(), QueryExecErr> {
        if self.is_match() {
            // pop output stack and treat it as a value!
            let top = self.output.pop().unwrap();
            self.output.last_mut().unwrap().push(top);
        }
        let top = self.state.pop();
        match top {
            Some(State::Sequence(pos, len)) => {
                debug_assert_eq!(pos, len);
                Ok(())
            }
            found => Err(QueryExecErr::InternalError(format!(
                "Bad exit_sequence state: {:?}",
                found
            ))),
        }
    }
    fn enter_map(&mut self) {
        if self.is_match() {
            self.output.push(OutputStackFrame::map());
        }
        self.state.push(State::StartMap);
    }
    fn exit_map(&mut self) {
        if self.is_match() && self.output.len() > 1 {
            // pop output stack and treat it as a value!
            let top = self.output.pop().unwrap();
            self.output.last_mut().unwrap().push(top);
        }
        let top = self.state.pop();
        debug_assert_eq!(top, Some(State::StartMap));
    }
    fn enter_map_key(&mut self) {
        self.state.push(State::MapKey);
    }
    fn exit_map_key(&mut self) -> Result<(), QueryExecErr> {
        // Leave MapKeyStr on state stack!
        match self.state.last() {
            Some(State::MapKeyStr(_)) => Ok(()),
            _ => Err(QueryExecErr::InternalError(format!(
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
    fn exit_map_value(&mut self) -> Result<(), QueryExecErr> {
        match self.state.pop() {
            Some(State::MapValue) => {}
            actual => {
                return Err(QueryExecErr::InternalError(format!(
                    "Expected MapValue state, found: {:?}",
                    actual
                )))
            }
        }
        match self.state.pop() {
            Some(State::MapKeyStr(name)) => {
                self.exit_name(Some(&name));
            }
            actual => {
                return Err(QueryExecErr::InternalError(format!(
                    "Expected MapKeyStr state, found: {:?}",
                    actual
                )))
            }
        }
        match self.state.pop() {
            Some(State::MapKey) => Ok(()),
            actual => Err(QueryExecErr::InternalError(format!(
                "Expected MapKey state, found: {:?}",
                actual
            ))),
        }
    }
}

/// An enum representing a runtime error given a correctly-parsed query.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum QueryExecErr {
    /// You gave us a query that has no fields or array accesses in it.
    /// Just call serde_json::to_value instead of going through the query API!
    EmptyQuery,
    /// Basically a panic; please open a [github issue](https://github.com/jjfiv/access-json/issues), with data if possible!
    InternalError(String),
    /// Since we're currently implementing a serde Serializer to run the queries, we need a catch-all for custom errors, e.g., in user-specified serialization targets.
    Serialization(String),
}

impl From<serde_json::Error> for QueryExecErr {
    fn from(err: serde_json::Error) -> Self {
        Self::InternalError(format!("{:?}", err))
    }
}

impl std::fmt::Display for QueryExecErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)?;
        Ok(())
    }
}
impl std::error::Error for QueryExecErr {
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

impl serde::ser::Error for QueryExecErr {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        // Note: erased_serde brings us here on error.
        QueryExecErr::Serialization(msg.to_string())
    }
}

impl<'a> serde::Serializer for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecErr;

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
            Some(State::MapKeyStr(_)) => Err(QueryExecErr::InternalError(
                "Shouldn't see multiple str for the same key!".into(),
            )),
            Some(_) => self.possible_result(&v),
            Option::None => Err(QueryExecErr::InternalError(
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

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        // see test_unit_struct
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        // For unit variants of enums, we/serde serialize them as just a String.
        // See test_enum_examples; Pet::Bird.
        self.serialize_str(variant)?;
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
        // See test_newtype_struct:
        // struct Meters(f64) is serialized as just a f64 and we don't care about the name of that type...?
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
            self.exit_name(Some(variant));
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
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.enter_map();
        self.must_enter_name(variant);
        self.enter_sequence(Some(len));
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
        self.enter_map();
        self.must_enter_name(variant);
        Ok(self)
    }
}

impl<'a> serde::ser::SerializeSeq for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecErr;

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
    type Error = QueryExecErr;
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
    type Error = QueryExecErr;
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
    type Error = QueryExecErr;
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
    type Error = QueryExecErr;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.sequence_element(value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_sequence()?;
        self.exit_map();
        self.exit_name(None);
        Ok(())
    }
}
impl<'a> serde::ser::SerializeStruct for &'a mut QueryExecutor {
    type Ok = ();
    type Error = QueryExecErr;
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
            self.exit_name(Some(key));
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
    type Error = QueryExecErr;
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
            self.exit_name(Some(key));
        }
        Ok(())
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit_map();
        self.exit_name(None);
        Ok(())
    }
}
