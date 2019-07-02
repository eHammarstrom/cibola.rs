use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JSONValue<'a> {
    Object(HashMap<&'a str, JSONValue<'a>>),
    Array(Vec<JSONValue<'a>>),
    Bool(bool),
    Text(&'a str),
    Number(f64),
    Null,
}

