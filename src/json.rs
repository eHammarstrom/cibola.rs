use crate::parse;
use std::convert::From;

use std::collections::HashMap;

/// Parse JSON string into recursive JSONValue struct
pub fn from_str(text: &str) -> Result<JSONValue, parse::Error> {
    let mut parse_context = parse::ParseContext::new(text);

    parse_context.parse()
}

#[derive(Debug, Clone, PartialEq)]
pub enum JSONValue {
    Object(HashMap<String, JSONValue>),
    Array(Vec<JSONValue>),
    Bool(bool),
    Text(String),
    Number(f64),
    Null,
}

impl From<HashMap<String, JSONValue>> for JSONValue {
    fn from(item: HashMap<String, JSONValue>) -> Self {
        JSONValue::Object(item)
    }
}

impl From<HashMap<&str, JSONValue>> for JSONValue {
    fn from(item: HashMap<&str, JSONValue>) -> Self {
        let map = item.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();

        JSONValue::Object(map)
    }
}

impl From<Vec<JSONValue>> for JSONValue {
    fn from(item: Vec<JSONValue>) -> Self {
        JSONValue::Array(item)
    }
}

impl From<bool> for JSONValue {
    fn from(item: bool) -> Self {
        JSONValue::Bool(item)
    }
}

impl From<f64> for JSONValue {
    fn from(item: f64) -> Self {
        JSONValue::Number(item)
    }
}

impl From<f32> for JSONValue {
    fn from(item: f32) -> Self {
        JSONValue::Number(item.into())
    }
}

impl From<String> for JSONValue {
    fn from(item: String) -> Self {
        JSONValue::Text(item)
    }
}

impl From<&str> for JSONValue {
    fn from(item: &str) -> Self {
        JSONValue::Text(item.to_owned())
    }
}

