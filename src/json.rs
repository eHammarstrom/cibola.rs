use crate::parse;

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Object<'a>(pub HashMap<&'a str, JSONData<'a>>);

#[derive(Debug, Clone, PartialEq)]
pub struct Array<'a>(pub Vec<JSONData<'a>>);

#[derive(Debug, Clone, PartialEq)]
pub enum JSON<'a> {
    Object(Object<'a>),
    Array(Array<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum JSONData<'a> {
    Object(Object<'a>),
    Array(Array<'a>),
    Bool(bool),
    Text(&'a str),
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

impl<'a, 'b: 'a> JSON<'a> {
    pub fn parse(text: &'a str) -> Result<JSON<'b>, JSONError> {
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
