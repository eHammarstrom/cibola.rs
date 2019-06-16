use crate::parse;

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Object(pub HashMap<String, JSONData>);

#[derive(Debug, Clone, PartialEq)]
pub struct Array(pub Vec<JSONData>);

#[derive(Debug, Clone, PartialEq)]
pub enum JSON {
    Object(Object),
    Array(Array),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JSONData {
    Object(Object),
    Array(Array),
    Bool(bool),
    Text(String),
    Number(f64),
    Null,
}

#[derive(Debug)]
pub enum JSONError {
    InvalidJSON(parse::ParseError, parse::ParseError),
}

impl fmt::Display for JSONError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JSONError::InvalidJSON(obj, arr) => {
                writeln!(f, "Invalid JSON format. Reason:")?;
                writeln!(f, "object: {}", obj)?;
                writeln!(f, "array: {}", arr)
            }
        }
    }
}

impl JSON {
    pub fn parse(text: &str) -> Result<JSON, JSONError> {
        let mut parse_context = parse::ParseContext::new(text);

        let obj = parse_context.object();
        let arr = parse_context.array();

        match (obj, arr) {
            (Ok(object), _) => Ok(JSON::Object(object)),
            (_, Ok(array)) => Ok(JSON::Array(array)),
            (Err(obj_err), Err(arr_err)) => Err(JSONError::InvalidJSON(obj_err, arr_err)),
        }
    }
}
