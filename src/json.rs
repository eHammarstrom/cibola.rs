use crate::parse;

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JSONValue {
    Object(HashMap<String, JSONValue>),
    Array(Vec<JSONValue>),
    Bool(bool),
    Text(String),
    Number(f64),
    Null,
}

pub fn from_str(text: &str) -> Result<JSONValue, parse::Error> {
    let mut parse_context = parse::ParseContext::new(text);

    parse_context.parse()
}

pub fn object(x: HashMap<String, JSONValue>) -> JSONValue
{
    JSONValue::Object(x)
}

pub fn array<T>(x: T) -> JSONValue
where
T: Into<Vec<JSONValue>>,
{
    JSONValue::Array(x.into())
}

pub fn bool<T>(x: T) -> JSONValue
where
T: Into<bool>,
{
    JSONValue::Bool(x.into())
}

pub fn text<T>(x: T) -> JSONValue
where
T: Into<String>,
{
    JSONValue::Text(x.into())
}

pub fn number<T>(x: T) -> JSONValue
where
T: Into<f64>,
{
    JSONValue::Number(x.into())
}

pub fn null() -> JSONValue
{
    JSONValue::Null
}

