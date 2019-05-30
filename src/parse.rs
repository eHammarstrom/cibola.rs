use crate::json;

use std::collections::HashMap;
use std::fmt;
use std::iter;
use std::str;

pub struct ParseContext<'a> {
    pos: (u32, u32),
    walk_pos: (u32, u32),
    peek_steps: u32,
    iter: str::Chars<'a>,
    text: &'a str,
}

pub enum ParseError {
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
        let ParseContext { pos, .. } = ctx;

        ParseError::UnexpectedToken {
            lineno: pos.0,
            col: pos.1,
            token: '\0',
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
            peek_steps: 0,
            iter: text.chars(),
            text,
        }
    }

    fn skip_char(&mut self, skip: char) -> bool {
        let mut did_skip = false;

        loop {
            match self.iter.next() {
                None => return false,
                Some(peek) => {
                    if peek != skip {
                        self.iter.next_back();
                        return did_skip;
                    }

                    did_skip = true;
                    self.peek_steps += 1;
                }
            }
        }
    }

    fn skip_whitespace(&mut self) -> bool {
        self.skip_char(' ')
    }

    fn skip_newline(&mut self) -> bool {
        self.skip_char('\n')
    }

    fn walk(&mut self, skip_ws_nl: bool) -> self::Result<char> {
        if skip_ws_nl {
            // skip whitespace or newline while we can
            while self.skip_whitespace() || self.skip_newline() {
                break;
            }
        };

        let peek = self.iter.next().ok_or(ParseError::EOS)?;

        self.peek_steps += 1;

        Ok(peek)
    }

    fn reverse(&mut self) {
        for _ in 0..self.peek_steps {
            let _ = self.iter.next_back();
        }
        self.peek_steps = 0;
    }

    fn fail<T>(&mut self, reason: &'static str) -> self::Result<T> {
        // reverse the iterator, we failed the parse
        self.reverse();
        Err(ParseError::unexpected_token(
            &self,
            "failed to parse object",
        ))
    }

    pub fn object(&mut self) -> self::Result<json::Object> {
        let peek = self.walk(true)?;

        let fields = match peek {
            '{' => self.fields(),
            _ => self.fail("failed to parse Object: missing {"),
        }?;

        let peek = self.walk(true)?;

        match peek {
            '}' => Ok(json::Object(fields)),
            _ => self.fail("failed to parse Object: missing }"),
        }
    }

    pub fn array(&mut self) -> self::Result<json::Array> {
        Err(ParseError::EOS)
    }

    fn fields(&mut self) -> self::Result<HashMap<String, json::JSONData>> {
        let mut hashmap = HashMap::<String, json::JSONData>::new();

        while let Ok((id, value)) = self.field() {
            let _ = hashmap.insert(id, value);

            // check if end of fields (object end })
            let peek = self.walk(true)?;
            if peek == '}' {
                self.reverse();
                break;
            }
        }

        // FIXME: check such that what follows is a valid token
        // otherwise the object was malformed

        Ok(hashmap)
    }

    fn field(&mut self) -> Result<(String, json::JSONData)> {
        // 1. parse identifier
        // 2. parse value

        let id = self.identifier()?;

        Ok(("".to_string(), json::JSONData::Null))
    }

    fn identifier(&mut self) -> Result<String> {
        Ok("lol".to_string())
    }
}
