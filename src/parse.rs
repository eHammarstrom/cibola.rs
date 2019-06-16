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
            self.head = Some(next);
            self.fail(format!("parse::eat expected '{}' but got '{}'", tok, next))
        }
    }

    fn eat_one_of(&mut self, match_chars: &[char]) -> self::Result<char> {
        let next = self.walk(true)?;

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

    fn eat_str(&mut self, mut match_str: String) -> self::Result<String> {
        let mut accumulator = String::new();

        // allow prefix spaces in front of first char
        let c = match_str.remove(0);
        self.eat(c, true)?;
        accumulator.push(c);

        for c in match_str.chars() {
            if self.eat(c, false).is_err() {
                return self.fail(format!(
                    "parse::eat_str expected '{}' but got '{}'",
                    c,
                    self.head.unwrap(),
                ));
            }
            accumulator.push(c);
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

    fn null(&mut self) -> Result<json::JSONData> {
        debug!("ParseContext::null");
        self.eat_str("null".to_string())?;
        Ok(json::JSONData::Null)
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
        let allowed_chars = [
            '.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'e', 'E',
        ];
        let mut accumulator = String::new();

        let negate = self.eat('-', true).is_ok();

        while let Ok(num) = self.eat_one_of(&allowed_chars[0..allowed_chars.len() - 2]) {
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

            // make sure negation is only used after exponent
            // then hand the negation over to `f64::from_str`
            if ['e', 'E'].contains(&num) && self.eat('-', false).is_ok() {
                accumulator.push('-');
            }
        }

        debug!("ParseContext::num> building {}", accumulator);

        let num = f64::from_str(&accumulator)
            .map(|num| if negate { -num } else { num })
            .map(json::JSONData::Number);

        match num {
            Ok(float) => Ok(float),
            Err(e) => self.fail(format!(
                "ParseContext::number> parse failed with: {}",
                e.to_string()
            )),
        }
    }

    fn value(&mut self) -> Result<json::JSONData> {
        self.null()
            .or_else(|_| self.text())
            .or_else(|_| self.boolean())
            .or_else(|_| self.number())
            .or_else(|_| self.text())
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
        let _ = env_logger::init();

        let n1 = "3.14";
        let n2 = "-3.14";
        let n3 = "23.2e-10";
        let n4 = "23.2E10";

        let mut c1 = parse::ParseContext::new(n1);
        let mut c2 = parse::ParseContext::new(n2);
        let mut c3 = parse::ParseContext::new(n3);
        let mut c4 = parse::ParseContext::new(n4);

        let r1 = c1.number();
        let r2 = c2.number();
        let r3 = c3.number();
        let r4 = c4.number();

        assert_eq!(json::JSONData::Number(3.14), r1.unwrap());
        assert_eq!(json::JSONData::Number(-3.14), r2.unwrap());
        assert_eq!(json::JSONData::Number(23.2e-10), r3.unwrap());
        assert_eq!(json::JSONData::Number(23.2E10), r4.unwrap());
    }

    #[test]
    fn parse_object() {
        let mut obj = HashMap::<String, json::JSONData>::new();
        obj.insert("myBool".to_string(), json::JSONData::Bool(true));
        obj.insert(
            "myString".to_string(),
            json::JSONData::Text("SomeString".to_string()),
        );

        let mut nest = obj.clone();

        nest.insert("myNumber".to_string(), json::JSONData::Number(33.14));
        nest.insert("myNull".to_string(), json::JSONData::Null);
        nest.insert("myNumber2".to_string(), json::JSONData::Number(-33.14));

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
                "myNull": null   ,
                "myNumber2": -33.14,
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
