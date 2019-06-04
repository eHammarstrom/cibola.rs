use crate::json;

use std::collections::HashMap;
use std::fmt;
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

        let next = self.iter.next().ok_or(ParseError::EOS)?;

        self.peek_steps += 1;

        Ok(next)
    }

    fn reverse(&mut self) {
        for _ in 0..self.peek_steps {
            let _ = self.iter.next_back();
        }
        self.peek_steps = 0;
    }

    fn consume(&mut self) {
        // point of no return
        // we 'consume' the peeked chars
        self.peek_steps = 0;
    }

    fn eat(&mut self, tok: char, skip_ws_nl: bool) -> self::Result<()> {
        let next = self.walk(skip_ws_nl)?;
        if next == tok {
            Ok(())
        } else {
            self.fail("parse::eat")
        }
    }

    fn eat_str(&mut self, match_str: &'static str) -> self::Result<String> {
        let mut next = self.walk(false)?;
        let mut accumulator = String::new();
        for c in match_str.chars() {
            if next != c {
                return self.fail(match_str);
            }

            next = self.walk(false)?;
        }

        Ok(accumulator)
    }

    fn eat_until(&mut self, tok: char) -> self::Result<String> {
        let mut next = self.walk(false)?;
        let mut accumulator = String::new();
        while next != tok {
            accumulator.push(next);
            next = self.walk(false)?;
        }
        Ok(accumulator)
    }

    fn fail<T>(&mut self, reason: &'static str) -> self::Result<T> {
        Err(ParseError::unexpected_token(
            &self,
            reason,
        ))
    }

    pub fn object(&mut self) -> self::Result<json::Object> {
        self.eat('{', true)?;
        let fields = self.fields()?;
        self.eat('}', true)?;
        Ok(json::Object(fields))
    }

    pub fn array(&mut self) -> self::Result<json::Array> {
        Err(ParseError::EOS)
    }

    fn fields(&mut self) -> self::Result<HashMap<String, json::JSONData>> {
        let mut hashmap = HashMap::<String, json::JSONData>::new();

        while let Ok((id, value)) = self.field() {
            let _ = hashmap.insert(id, value);
        }

        Ok(hashmap)
    }

    fn field(&mut self) -> Result<(String, json::JSONData)> {
        // 1. parse identifier
        // 2. parse value

        let id = self.string()?;
        self.eat(':', true)?;
        let val = self.value()?;

        Ok(( id, val ))
    }

    fn string(&mut self) -> Result<String> {
        self.eat('"', true)?;
        self.eat_until('"')
    }

    fn text(&mut self) -> Result<json::JSONData> {
        let s = self.string()?;
        Ok(json::JSONData::Text(s))
    }

    fn boolean(&mut self) -> Result<json::JSONData> {
        if let Ok(_) = self.eat_str("true") {
            Ok(json::JSONData::Bool(true))
        } else if let Ok(_) = self.eat_str("false") {
            Ok(json::JSONData::Bool(false))
        } else {
            self.fail("boolean")
        }
    }

    fn value(&mut self) -> Result<json::JSONData> {
        self.text()
            .or(self.boolean())
    }
}
