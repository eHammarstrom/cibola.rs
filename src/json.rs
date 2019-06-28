use crate::parse;

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

impl<'a, 'b: 'a> JSONValue<'a> {
    pub fn parse(text: &'a str) -> Result<JSONValue<'b>, parse::Error> {
        let mut parse_context = parse::ParseContext::new(text);

        parse_context.value()
    }
}
