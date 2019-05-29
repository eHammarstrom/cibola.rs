use crate::parse;

use std::collections::HashMap;
use std::fmt;

pub struct Object(pub HashMap<String, JSONData>);
pub struct Array(pub Vec<JSONData>);

pub enum JSON {
    Object(Object),
    Array(Array),
}

pub enum JSONData {
    Object(Object),
    Array(Array),
    Bool(bool),
    Text(String),
    Number(f64),
    Null,
}

enum JSONError {
    UnexpectedToken((u32, u32), char),
}

impl fmt::Display for JSONError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::JSONError::*;

        match self {
            UnexpectedToken((line, col), token) => write!(
                f,
                "Unexpected token '{}' at line {} col {}.",
                token, col, line
            ),
        }
    }
}

impl JSON {
    fn parse(text: &str) -> Result<JSON, JSONError> {
        let mut parse_context = parse::ParseContext::new(text);
        // hehe
        Ok(JSON::Array(Array(vec![JSONData::Null])))
    }
}
