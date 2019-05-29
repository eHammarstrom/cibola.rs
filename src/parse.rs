use crate::json;

use std::collections::HashMap;
use std::fmt;

pub struct ParseContext<'a> {
    pos: (u32, u32),
    walk_pos: (u32, u32),
    nom: Vec<char>,
    text: &'a str,
}

enum ParseError {
    EOS,
    UnexpectedToken {
        lineno: u32,
        col: u32,
        token: char,
        reason: &'static str,
    },
}

impl ParseError {
    fn unexpected_token(ctx: &ParseContext, reason: &'static str) -> ParseError {
        let ParseContext { pos, nom, .. } = ctx;

        let token = **nom.last().get_or_insert(&'\0');

        ParseError::UnexpectedToken {
            lineno: pos.0,
            col: pos.1,
            token,
            reason,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

type Result<T> = std::result::Result<T, ParseError>;

impl<'a> ParseContext<'a> {
    pub fn new(text: &str) -> ParseContext {
        ParseContext {
            pos: (0, 0),
            walk_pos: (0, 0),
            nom: Vec::new(),
            text,
        }
    }

    fn walk(&mut self, skip_ws: bool) -> self::Result<char> {
        let (mut lineno, mut col) = self.walk_pos;
        let mut txt_iter = self.text.chars().peekable();
        let mut peek;

        if skip_ws {
            loop {
                peek = txt_iter.peek().ok_or(ParseError::EOS)?;

                match peek {
                    ' ' => lineno += 1,
                    _ => break,
                }
            }
        } else {
            peek = txt_iter.peek().ok_or(ParseError::EOS)?;
        }

        self.nom.push(*peek);
        Ok(*peek)
    }

    fn fail<T>(&mut self, reason: &'static str) -> self::Result<T> {
        Err(ParseError::unexpected_token(
            &self,
            "failed to parse object",
        ))
    }

    pub fn object(&mut self) -> self::Result<json::Object> {
        let peek = self.walk(true)?;

        let fields = match peek {
            '{' => self.fields(),
            _ => self.fail("failed to parse Object"),
        }?;

        let peek = self.walk(true)?;

        match peek {
            '}' => Ok(json::Object(fields)),
            _ => self.fail("failed to parse Object"),
        }
    }

    pub fn fields(&mut self) -> self::Result<HashMap<String, json::JSONData>> {
        Ok(HashMap::new())
    }

    pub fn field(&mut self) -> Result<(String, json::JSONData)> {
        Ok(("".to_string(), json::JSONData::Null))
    }
}
