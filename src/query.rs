use erased_serde::Serialize as AnySerializable;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum QueryElement {
    AccessField { field: String },
    AccessArrayItem { name: String, index: usize },
    // TODO: AccessArrayLength { name: String },
}

impl QueryElement {
    pub fn field(field: &str) -> Self {
        Self::AccessField {
            field: field.into(),
        }
    }
    pub fn array_item(field: &str, index: usize) -> Self {
        Self::AccessArrayItem {
            name: field.into(),
            index,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct JSONQuery {
    pub elements: Vec<QueryElement>,
}

impl JSONQuery {
    pub fn new(elements: Vec<QueryElement>) -> Self {
        Self { elements }
    }
    pub fn execute(&self, target: &dyn AnySerializable) -> Option<serde_json::Value> {
        if self.elements.is_empty() {
            None
        } else {
            None
            //self.execute_recursive(target, self.elements[0], self.elements[1..])
        }
    }
}
