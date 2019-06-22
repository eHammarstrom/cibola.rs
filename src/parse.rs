use crate::json;

use std::collections::HashMap;
use std::f64;
use std::fmt;
use std::slice;
use std::str;
use std::str::FromStr;

#[derive(Debug)]
pub struct ParseContext<'a> {
    line: u32,
    col: u32,
    iter: str::Chars<'a>,
    head: Option<char>,
    text: &'a str,
    index: usize, // index in source str `text`
}

#[derive(Debug)]
pub enum ParseError {
    EOS,
    UnexpectedToken {
        line: u32,
        col: u32,
        token: char,
        reason: &'static str,
    },
}

impl ParseError {
    fn unexpected_token(ctx: &ParseContext, reason: &'static str) -> ParseError {
        let ParseContext {
            line,
            col,
            head,
            // iter,
            ..
        } = ctx;

        ParseError::UnexpectedToken {
            line: *line,
            col: *col,
            token: head.unwrap_or(' '),
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

impl<'a, 'b: 'a> ParseContext<'a> {
    pub fn new(text: &'b str) -> ParseContext {
        ParseContext {
            line: 0,
            col: 0,
            iter: text.chars(),
            head: None,
            text,
            index: 0,
        }
    }

    #[inline]
    fn current_byte_as_ptr(&self) -> *const u8 {
        let p: *const u8 = self.text.as_bytes().as_ptr();

        unsafe { p.offset(self.index as isize) }
    }

    #[inline]
    fn current_char(&mut self) -> Option<char> {
        self.head.or_else(|| self.iter.next())
    }

    #[inline]
    fn add_lines(&mut self, num: u32) {
        self.line += num;
        self.col = 0;
    }

    #[inline]
    fn skip_char(&mut self, skip: char) -> (u32, bool) {
        let char_len = skip.len_utf8();
        let mut did_skip = false;
        let mut skips = 0;

        loop {
            let current = self.current_char();

            match current {
                None => return (0, false),
                Some(peek) => {
                    if peek != skip {
                        self.head = Some(peek);
                        self.index += char_len * skips as usize;
                        return (skips, did_skip);
                    }

                    self.head = None;
                    did_skip = true;
                    skips += 1;
                }
            }
        }
    }

    #[inline]
    fn skip_whitespace(&mut self) -> bool {
        let (skips, skipped) = self.skip_char(' ');

        self.col += skips;

        skipped
    }

    #[inline]
    fn skip_newline(&mut self) -> bool {
        let (skips, skipped) = self.skip_char('\n');

        self.add_lines(skips);

        skipped
    }

    #[inline]
    fn skip_tab(&mut self) -> bool {
        let (skips, skipped) = self.skip_char('\t');

        self.col += skips;

        skipped
    }

    #[inline]
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

    #[inline]
    fn ate(&mut self, c: char) {
        self.head = None;
        self.index += c.len_utf8();

        if c == '\n' {
            self.add_lines(1);
        } else {
            self.col += 1;
        }
    }

    #[inline]
    fn eat(&mut self, tok: char, skip_ws_nl: bool) -> self::Result<()> {
        let next = self.walk(skip_ws_nl)?;

        if next == tok {
            self.ate(next);

            Ok(())
        } else {
            self.head = Some(next);
            self.fail("parse::eat")
        }
    }

    #[inline]
    fn eat_one_of(&mut self, match_chars: &[char]) -> self::Result<char> {
        let next = self.walk(true)?;

        if match_chars.contains(&next) {
            self.ate(next);
            Ok(next)
        } else {
            self.head = Some(next);
            self.fail("parse::eat_one_of")
        }
    }

    #[inline]
    fn eat_str(&mut self, match_str: &'static str) -> self::Result<&str> {
        let mut match_iter = match_str.chars();

        // allow prefix spaces in front of first char
        let c = match_iter.next().unwrap();
        self.eat(c, true)?;

        for c in match_iter {
            if self.eat(c, false).is_err() {
                return self.fail("parse::eat_str");
            }
        }

        // only create String if successful parse
        Ok(match_str)
    }

    #[inline]
    fn eat_until(&mut self, tok: char) -> self::Result<&'b str> {
        let ptr_start = self.current_byte_as_ptr();
        let idx_start = self.index;
        let mut next = self.walk(false)?;

        while next != tok {
            self.ate(next);
            next = self.walk(false)?;
        }

        let idx_end = self.index;
        self.head = Some(next);

        unsafe {
            Ok(str::from_utf8_unchecked(slice::from_raw_parts(
                ptr_start,
                idx_end - idx_start,
            )))
        }
    }

    #[inline]
    fn fail<T>(&mut self, reason: &'static str) -> self::Result<T> {
        Err(ParseError::unexpected_token(&self, reason))
    }

    #[inline]
    pub fn object(&mut self) -> self::Result<json::Object<'b>> {
        self.eat('{', true)?;
        let fields = self.fields()?;
        self.eat('}', true)?;
        Ok(json::Object(fields))
    }

    #[inline]
    fn fields(&mut self) -> self::Result<HashMap<&'b str, json::JSONData<'b>>> {
        let mut hashmap = HashMap::<&str, json::JSONData<'b>>::new();

        while let Ok((id, value)) = self.field() {
            let _ = hashmap.insert(id, value);
        }

        Ok(hashmap)
    }

    #[inline]
    fn field(&mut self) -> Result<(&'b str, json::JSONData<'b>)> {
        // 1. parse identifier
        // 2. parse value

        let id = self.string()?;
        self.eat(':', true)?;
        let val = self.value()?;
        // commas may trail
        let _ = self.eat(',', true);

        Ok((id, val))
    }

    #[inline]
    pub fn array(&mut self) -> self::Result<json::Array<'b>> {
        self.eat('[', true)?;
        let values = self.values()?;
        self.eat(']', true)?;

        Ok(json::Array(values))
    }

    #[inline]
    fn values(&mut self) -> self::Result<Vec<json::JSONData<'b>>> {
        let mut vals = Vec::<json::JSONData<'b>>::new();

        while let Ok(v) = self.value() {
            let _ = vals.push(v);
            // commas may trail
            let _ = self.eat(',', true);
        }

        Ok(vals)
    }

    #[inline]
    fn string(&mut self) -> Result<&'b str> {
        self.eat('"', true)?;
        let s = self.eat_until('"')?;
        self.eat('"', false)?;

        Ok(s)
    }

    #[inline]
    fn null(&mut self) -> Result<json::JSONData<'b>> {
        self.eat_str("null")?;
        Ok(json::JSONData::Null)
    }

    #[inline]
    fn text(&mut self) -> Result<json::JSONData<'b>> {
        let s = self.string()?;
        Ok(json::JSONData::Text(s))
    }

    #[inline]
    fn boolean(&mut self) -> Result<json::JSONData<'b>> {
        if let Ok(_) = self.eat_str("true") {
            Ok(json::JSONData::Bool(true))
        } else if let Ok(_) = self.eat_str("false") {
            Ok(json::JSONData::Bool(false))
        } else {
            self.fail("parse::boolean")
        }
    }

    #[inline]
    fn number(&mut self) -> Result<json::JSONData<'b>> {
        let ptr_start = self.current_byte_as_ptr();
        let idx_start = self.index;

        let allowed_chars = [
            '-', '.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'e', 'E',
        ];

        // eat through valid bytes
        while let Ok(_) = self.eat_one_of(&allowed_chars) {}

        let idx_end = self.index;

        let num = unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(ptr_start, idx_end - idx_start))
        };

        let num = f64::from_str(&num).map(json::JSONData::Number);

        match num {
            Ok(float) => Ok(float),
            _ => self.fail("parse::number"),
        }
    }

    #[inline]
    fn value(&mut self) -> Result<json::JSONData<'b>> {
        self.null()
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
    use std::collections::HashMap;

    #[test]
    fn parse_text_and_boolean() {
        let mut obj = HashMap::<&str, json::JSONData>::new();
        obj.insert("myBool", json::JSONData::Bool(true));
        obj.insert("myString", json::JSONData::Text("SomeString"));

        let txt = r#"{ "myString": "SomeString", "myBool":  true }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::Object(obj));
    }

    #[test]
    fn parse_text_and_boolean_trailing_comma() {
        let mut obj = HashMap::<&str, json::JSONData>::new();
        obj.insert("myBool", json::JSONData::Bool(true));
        obj.insert("myString", json::JSONData::Text("SomeString"));

        let txt = r#"{ "myString": "SomeString", "myBool":  true, }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::Object(obj));
    }

    #[test]
    fn parse_nested_object() {
        let mut obj = HashMap::<&str, json::JSONData>::new();
        obj.insert("myBool", json::JSONData::Bool(true));
        obj.insert("myString", json::JSONData::Text("SomeString"));
        let nest = obj.clone();
        obj.insert("myObject", json::JSONData::Object(json::Object(nest)));

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
        let mut obj = HashMap::<&str, json::JSONData>::new();
        obj.insert("myBool", json::JSONData::Bool(true));
        obj.insert("myString", json::JSONData::Text("SomeString"));

        let mut nest = obj.clone();

        nest.insert("myNumber", json::JSONData::Number(33.14));
        nest.insert("myNull", json::JSONData::Null);
        nest.insert("myNumber2", json::JSONData::Number(-33.14));

        obj.insert("myObject", json::JSONData::Object(json::Object(nest)));

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
        let mut map = HashMap::<&str, json::JSONData>::new();
        map.insert("myBool", json::JSONData::Bool(true));
        map.insert("myString", json::JSONData::Text("SomeString"));

        let obj = json::Object(map);

        let arr = vec![
            json::JSONData::Text("SomeString"),
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
