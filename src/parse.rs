use crate::json;

use log::debug;
use std::collections::HashMap;
use std::f64;
use std::fmt;
use std::str;
use std::str::FromStr;

#[derive(Debug)]
pub struct ParseContext<'a> {
    line: u32,
    col: u32,
    iter: str::Chars<'a>,
    head: Option<char>,
    text: &'a str,
}

#[derive(Debug)]
pub enum ParseError {
    EOS,
    UnexpectedToken {
        line: u32,
        col: u32,
        token: char,
        reason: String,
    },
}

impl ParseError {
    fn unexpected_token(ctx: &ParseContext, reason: String) -> ParseError {
        let ParseContext { line, col, .. } = ctx;

        ParseError::UnexpectedToken {
            line: *line,
            col: *col,
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
            line: 0,
            col: 0,
            iter: text.chars(),
            head: None,
            text,
        }
    }

    fn add_lines(&mut self, num: u32) {
        self.line += num;
        self.col = 0;
    }

    fn skip_char(&mut self, skip: char) -> (u32, bool) {
        let mut did_skip = false;
        let mut skips = 0;

        loop {
            let current = self.head.or_else(|| self.iter.next());

            match current {
                None => return (0, false),
                Some(peek) => {
                    if peek != skip {
                        if did_skip {
                            debug!("ParseContext::skip_char> '{}'", skip);
                        }

                        self.head = Some(peek);
                        return (skips, did_skip);
                    }

                    self.head = None;
                    did_skip = true;
                    skips += 1;
                }
            }
        }
    }

    fn skip_whitespace(&mut self) -> bool {
        let (skips, skipped) = self.skip_char(' ');

        self.col += skips;

        skipped
    }

    fn skip_newline(&mut self) -> bool {
        let (skips, skipped) = self.skip_char('\n');

        self.add_lines(skips);

        skipped
    }

    fn skip_tab(&mut self) -> bool {
        let (skips, skipped) = self.skip_char('\t');

        self.col += skips;

        skipped
    }

    fn walk(&mut self, skip_ws_nl: bool) -> self::Result<char> {
        if skip_ws_nl {
            // skip whitespace, newline, and tab while we can
            loop {
                let skipped = self.skip_whitespace() || self.skip_newline() || self.skip_tab();

                if !skipped {
                    break;
                }
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
            debug!(
                "ParseContext::eat> clear head, was {}",
                self.head.unwrap_or('\0')
            );

            self.head = None;

            if next == '\n' {
                self.add_lines(1);
            } else {
                self.col += 1;
            }

            Ok(())
        } else {
            self.fail(format!("parse::eat expected '{}' but got '{}'", tok, next))
        }
    }

    fn eat_one_of(&mut self, match_chars: &[char]) -> self::Result<char> {
        let mut next = self.walk(true)?;

        if match_chars.contains(&next) {
            self.head = None;
            Ok(next)
        } else {
            self.head = Some(next);
            self.fail(format!(
                "ParseContext::eat_one_of> expected one of {:?} but got {}",
                match_chars, next,
            ))
        }
    }

    fn eat_str(&mut self, match_str: String) -> self::Result<String> {
        let mut accumulator = String::new();

        for c in match_str.chars() {
            if self.eat(c, false).is_ok() {
                accumulator.push(c);
            } else {
                return self.fail(format!(
                    "parse::eat_str expected '{}' but got '{}'",
                    c,
                    self.head.unwrap(),
                ));
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
        debug!("ParseContext::object");
        self.eat('{', true)?;
        let fields = self.fields()?;
        self.eat('}', true)?;
        Ok(json::Object(fields))
    }

    fn fields(&mut self) -> self::Result<HashMap<String, json::JSONData>> {
        debug!("ParseContext::fields");
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
        // commas may trail
        let _ = self.eat(',', true);

        Ok((id, val))
    }

    pub fn array(&mut self) -> self::Result<json::Array> {
        debug!("ParseContext::array");

        self.eat('[', true)?;
        let values = self.values()?;
        self.eat(']', true)?;

        Ok(json::Array(values))
    }

    fn values(&mut self) -> self::Result<Vec<json::JSONData>> {
        debug!("ParseContext::values");
        let mut vals = Vec::<json::JSONData>::new();

        while let Ok(v) = self.value() {
            let _ = vals.push(v);
            // commas may trail
            let _ = self.eat(',', true);
        }

        Ok(vals)
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

    fn number(&mut self) -> Result<json::JSONData> {
        let allowed_chars = ['.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        let mut accumulator = String::new();

        while let Ok(num) = self.eat_one_of(&allowed_chars) {
            debug!("ParseContext::num> got {}", num);
            accumulator.push(num);

            // break on separator to continue parsing nums only
            if num == '.' {
                break;
            }
        }

        while let Ok(num) = self.eat_one_of(&allowed_chars[1..]) {
            debug!("ParseContext::num> got {}", num);
            accumulator.push(num);
        }

        debug!("ParseContext::num> building {}", accumulator);

        let num = f64::from_str(&accumulator).map(json::JSONData::Number);

        match num {
            Ok(float) => Ok(float),
            Err(e) => self.fail(format!(
                "ParseContext::number> parse failed with: {}",
                e.to_string()
            )),
        }
    }

    fn value(&mut self) -> Result<json::JSONData> {
        self.text()
            .or_else(|_| self.boolean())
            .or_else(|_| self.number())
            .or_else(|_| self.array().map(json::JSONData::Array))
            .or_else(|_| self.object().map(json::JSONData::Object))
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

    #[test]
    fn parse_nested_object() {
        let mut obj = HashMap::<String, json::JSONData>::new();
        obj.insert("myBool".to_string(), json::JSONData::Bool(true));
        obj.insert(
            "myString".to_string(),
            json::JSONData::Text("SomeString".to_string()),
        );
        let nest = obj.clone();
        obj.insert(
            "myObject".to_string(),
            json::JSONData::Object(json::Object(nest)),
        );

        let txt = r#"

        {   "myString": "SomeString",
            "myBool":  true,
            "myObject": {
                "myString": "SomeString",
                "myBool": true,
            },
        }
        "#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::Object(obj));
    }

    #[test]
    fn parse_number() {
        let mut obj = HashMap::<String, json::JSONData>::new();
        obj.insert("myBool".to_string(), json::JSONData::Bool(true));
        obj.insert(
            "myString".to_string(),
            json::JSONData::Text("SomeString".to_string()),
        );

        let mut nest = obj.clone();

        nest.insert("myNumber".to_string(), json::JSONData::Number(33.14));

        obj.insert(
            "myObject".to_string(),
            json::JSONData::Object(json::Object(nest)),
        );

        let txt = r#"

        {   "myString": "SomeString",
            "myBool":  true,
            "myObject": {
                "myString": "SomeString",
                "myBool": true,
                "myNumber": 33.14,
            },
        }
        "#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::Object(obj));
    }

    #[test]
    fn parse_array() {
        let mut map = HashMap::<String, json::JSONData>::new();
        map.insert("myBool".to_string(), json::JSONData::Bool(true));
        map.insert(
            "myString".to_string(),
            json::JSONData::Text("SomeString".to_string()),
        );

        let obj = json::Object(map);

        let arr = vec![
            json::JSONData::Text("SomeString".to_string()),
            json::JSONData::Object(obj),
            json::JSONData::Number(33.14),
        ];

        let txt = r#"

        ["SomeString",
                { "myBool": true, "myString": "SomeString", },

           33.14,]

        "#;

        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.array();

        assert_eq!(res.unwrap(), json::Array(arr));
    }
}
