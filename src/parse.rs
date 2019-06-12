use crate::json;

use log::debug;
use std::collections::HashMap;
use std::fmt;
use std::str;

#[derive(Debug)]
pub struct ParseContext<'a> {
    pos: (u32, u32),
    walk_pos: (u32, u32),
    iter: str::Chars<'a>,
    head: Option<char>,
    text: &'a str,
}

#[derive(Debug)]
pub enum ParseError {
    EOS,
    UnexpectedToken {
        lineno: u32,
        col: u32,
        token: char,
        reason: String,
    },
}

impl ParseError {
    fn unexpected_token(ctx: &ParseContext, reason: String) -> ParseError {
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
            iter: text.chars(),
            head: None,
            text,
        }
    }

    fn skip_char(&mut self, skip: char) -> bool {
        let mut did_skip = false;

        loop {
            let current = self.head.or_else(|| self.iter.next());

            match current {
                None => return false,
                Some(peek) => {
                    if peek != skip {
                        self.head = Some(peek);
                        return did_skip;
                    }

                    self.head = None;
                    did_skip = true;
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

        let next = self
            .head
            .or_else(|| self.iter.next())
            .ok_or(ParseError::EOS)?;

        Ok(next)
    }

    fn eat(&mut self, tok: char, skip_ws_nl: bool) -> self::Result<()> {
        let next = self.walk(skip_ws_nl)?;

        debug!("ParseContext::eat> eat if {} == {}", tok, next);

        if next == tok {
            self.head = None;
            Ok(())
        } else {
            self.fail(format!("parse::eat expected '{}' but got '{}'", tok, next))
        }
    }

    fn eat_str(&mut self, match_str: String) -> self::Result<String> {
        let mut accumulator = String::new();

        for c in match_str.chars() {
            if self.eat(c, false).is_ok() {
                accumulator.push(c);
            }
        }

        Ok(accumulator)
    }

    fn eat_until(&mut self, tok: char) -> self::Result<String> {
        debug!("ParseContext::eat_until> {}", tok);
        let mut next = self.walk(false)?;
        let mut accumulator = String::new();

        while next != tok {
            debug!("ParseContext::eat_until> nom {} != {}", next, tok);
            accumulator.push(next);
            next = self.walk(false)?;
        }

        self.head = Some(next);

        Ok(accumulator)
    }

    fn fail<T>(&mut self, reason: String) -> self::Result<T> {
        Err(ParseError::unexpected_token(&self, reason))
    }

    pub fn object(&mut self) -> self::Result<json::Object> {
        debug!("{:#?}", self);
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

        debug!("ParseContext::field");
        let id = self.string()?;
        debug!("ParseContext::field> id = {}", id);
        self.eat(':', true)?;
        let val = self.value()?;
        debug!("ParseContext::field> value = {:?}", val);
        let _ = self.eat(',', true);

        Ok((id, val))
    }

    fn string(&mut self) -> Result<String> {
        debug!("ParseContext::string");
        self.eat('"', true)?;
        debug!("ParseContext::string> nom nom nom");
        let s = self.eat_until('"')?;
        debug!("ParseContext::string> {}", s);
        self.eat('"', false)?;
        Ok(s)
    }

    fn text(&mut self) -> Result<json::JSONData> {
        debug!("ParseContext::text");
        let s = self.string()?;
        debug!("ParseContext::text> {}", s);
        Ok(json::JSONData::Text(s))
    }

    fn boolean(&mut self) -> Result<json::JSONData> {
        debug!("ParseContext::boolean");
        if let Ok(_) = self.eat_str("true".to_string()) {
            Ok(json::JSONData::Bool(true))
        } else if let Ok(_) = self.eat_str("false".to_string()) {
            Ok(json::JSONData::Bool(false))
        } else {
            self.fail("boolean".to_string())
        }
    }

    fn value(&mut self) -> Result<json::JSONData> {
        self.text().or_else(|_| self.boolean())
    }
}

#[cfg(test)]
mod tests {
    use crate::json;
    use crate::parse;
    use env_logger;
    use std::collections::HashMap;

    #[test]
    fn parse_text_and_boolean() {
        let _ = env_logger::try_init();

        let mut obj = HashMap::<String, json::JSONData>::new();
        obj.insert("myBool".to_string(), json::JSONData::Bool(true));
        obj.insert(
            "myString".to_string(),
            json::JSONData::Text("SomeString".to_string()),
        );

        let txt = r#"{ "myString": "SomeString", "myBool":  true }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::Object(obj));
    }

    #[test]
    fn parse_text_and_boolean_trailing_comma() {
        let _ = env_logger::try_init();

        let mut obj = HashMap::<String, json::JSONData>::new();
        obj.insert("myBool".to_string(), json::JSONData::Bool(true));
        obj.insert(
            "myString".to_string(),
            json::JSONData::Text("SomeString".to_string()),
        );

        let txt = r#"{ "myString": "SomeString", "myBool":  true, }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::Object(obj));
    }
}
