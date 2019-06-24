use crate::json;

use std::collections::HashMap;
use std::f64;
use std::fmt;
use std::slice;
use std::str;
use std::str::FromStr;

//
// Borrowed from: https://github.com/maciejhirsz/json-rust/blob/master/src/parser.rs#L158
//
// Look up table that marks which characters are allowed in their raw
// form in a string.
const QU: bool = false; // double quote       0x22
const BS: bool = false; // backslash          0x5C
const CT: bool = false; // control character  0x00 ... 0x1F
const __: bool = true;

static ALLOWED: [bool; 256] = [
    // 0   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 0
    CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, CT, // 1
    __, __, QU, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
    __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
];

#[derive(Debug)]
pub struct ParseContext<'a> {
    line: u32,
    col: u32,
    bytes: &'a [u8],
    ate: u8,
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
            bytes,
            index,
            ..
        } = ctx;

        ParseError::UnexpectedToken {
            line: *line,
            col: *col,
            token: bytes[*index] as char,
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
        #[cfg(test)]
        println!("---- TEXT START ----\n{}\n---- TEXT END ----", text);
        ParseContext {
            line: 0,
            col: 0,
            bytes: text.as_bytes(),
            ate: b'\x00',
            text,
            index: 0,
        }
    }

    /// Returns pointer to current byte index
    fn current_byte_as_ptr(&self) -> *const u8 {
        let p: *const u8 = self.bytes.as_ptr();
        unsafe { p.offset(self.index as isize) }
    }

    /// Returns byte at index or EOF (as None)
    fn current_byte(&self) -> Option<u8> {
        if self.index < self.bytes.len() {
            Some(self.bytes[self.index])
        } else {
            None
        }
    }

    /// Increases line count by N, and resets column position
    fn add_lines(&mut self, num: u32) {
        self.line += num;
        self.col = 0;
    }

    #[inline(always)]
    fn peek(&self) -> Option<u8> {
        if self.index + 1 < self.bytes.len() {
            Some(self.bytes[self.index + 1])
        } else {
            None
        }
    }

    #[inline(always)]
    fn accept(&mut self) {
        self.index += 1;
    }

    /// Returns the number of skipped bytes, given the skip byte
    fn skip_byte(&mut self, skip: u8) -> (u32, bool) {
        let mut did_skip = false;
        let mut skips = 0;

        loop {
            let current_byte = self.current_byte();

            match current_byte {
                None => return (0, false),
                Some(byte) => {
                    if byte != skip {
                        return (skips, did_skip);
                    } else {
                        self.ate(byte);
                        did_skip = true;
                        skips += 1;
                    }
                }
            }
        }
    }

    fn skip_whitespace(&mut self) -> bool {
        let (skips, skipped) = self.skip_byte(b' ');

        self.col += skips;

        skipped
    }

    fn skip_newline(&mut self) -> bool {
        let (skips, skipped) = self.skip_byte(b'\n');

        self.add_lines(skips);

        skipped
    }

    fn skip_tab(&mut self) -> bool {
        let (skips, skipped) = self.skip_byte(b'\t');

        self.col += skips;

        skipped
    }

    fn walk(&mut self, allow_skip: bool) -> self::Result<u8> {
        if allow_skip {
            // skip whitespace, newline, and tab while we can
            loop {
                let skipped = self.skip_whitespace() || self.skip_newline() || self.skip_tab();

                if !skipped {
                    break;
                }
            }
        };

        let next = self.current_byte().ok_or(ParseError::EOS)?;

        // because we have already skipped whitespace, newline, and tab
        // we may now coerce b'\' + b'\t' to b'\t'
        //
        // escape = '"' '\' '/' 'b' 'f' 'n' 'r' 't' ('u' hex hex hex hex)
        //
        // and we know that !ALLOWED[byte] = escape
        //
        let next = match (next, self.peek()) {
            // escape char and control character
            (b'\\', Some(peek)) if !ALLOWED[peek as usize] => {
                self.ate(next);

                // TODO: add unicode parse
                match peek {
                    b'r' => b'\r',
                    b'n' => b'\n',
                    b't' => b'\t',
                    // f b " \ / u
                    _ => peek,
                }
            }
            // no peek or escape
            _ => next,
        };

        Ok(next)
    }

    fn ate(&mut self, b: u8) {
        self.accept();

        self.ate = b;

        if b == b'\n' {
            self.add_lines(1);
        } else {
            self.col += 1;
        }
    }

    fn eat(&mut self, token: u8, skip_ws_nl: bool) -> self::Result<()> {
        let next = self.walk(skip_ws_nl)?;

        if next == token {
            self.ate(next);

            Ok(())
        } else {
            self.fail("parse::eat")
        }
    }

    fn eat_one_of(&mut self, match_chars: &[u8]) -> self::Result<u8> {
        let next = self.walk(false)?;

        if match_chars.contains(&next) {
            self.ate(next);
            Ok(next)
        } else {
            self.fail("parse::eat_one_of")
        }
    }

    fn eat_str(&mut self, match_str: &'static str) -> self::Result<&str> {
        let match_bytes = match_str.as_bytes();

        // allow prefix spaces in front of first char
        let b = match_bytes[0];
        self.eat(b, true)?;

        for b in &match_bytes[1..] {
            if self.eat(*b, false).is_err() {
                return self.fail("parse::eat_str");
            }
        }

        // only create String if successful parse
        Ok(match_str)
    }

    fn eat_until(&mut self, token: u8) -> self::Result<&'b str> {
        let ptr_start = self.current_byte_as_ptr();
        let idx_start = self.index;
        let mut next = self.walk(false)?;

        while next != token {
            self.ate(next);
            next = self.walk(false)?;
        }

        let idx_end = self.index;
        // do-while :(
        // self.backtrack();

        unsafe {
            Ok(str::from_utf8_unchecked(slice::from_raw_parts(
                ptr_start,
                idx_end - idx_start,
            )))
        }
    }

    fn fail<T>(&mut self, reason: &'static str) -> self::Result<T> {
        Err(ParseError::unexpected_token(&self, reason))
    }

    pub fn object(&mut self) -> self::Result<json::Object<'b>> {
        self.eat(b'{', true)?;
        let fields = self.fields()?;
        self.eat(b'}', true)?;
        Ok(json::Object(fields))
    }

    fn fields(&mut self) -> self::Result<HashMap<&'b str, json::JSONData<'b>>> {
        let mut hashmap = HashMap::<&str, json::JSONData<'b>>::new();

        while let Ok((id, value)) = self.field() {
            let _ = hashmap.insert(id, value);
        }

        Ok(hashmap)
    }

    fn field(&mut self) -> Result<(&'b str, json::JSONData<'b>)> {
        // 1. parse identifier
        // 2. parse value

        let id = self.string()?;
        self.eat(b':', true)?;
        let val = self.value()?;
        // commas may trail
        let _ = self.eat(b',', true);

        Ok((id, val))
    }

    pub fn array(&mut self) -> self::Result<json::Array<'b>> {
        self.eat(b'[', true)?;
        let values = self.values()?;
        self.eat(b']', true)?;

        Ok(json::Array(values))
    }

    fn values(&mut self) -> self::Result<Vec<json::JSONData<'b>>> {
        let mut vals = Vec::<json::JSONData<'b>>::new();

        while let Ok(v) = self.value() {
            let _ = vals.push(v);
            // commas may trail
            let _ = self.eat(b',', true);
        }

        Ok(vals)
    }

    fn string(&mut self) -> Result<&'b str> {
        self.eat(b'"', true)?;
        let s = self.eat_until(b'"')?;
        self.eat(b'"', false)?;

        Ok(s)
    }

    fn null(&mut self) -> Result<json::JSONData<'b>> {
        self.eat_str("null")?;
        Ok(json::JSONData::Null)
    }

    fn text(&mut self) -> Result<json::JSONData<'b>> {
        let s = self.string()?;
        Ok(json::JSONData::Text(s))
    }

    fn boolean(&mut self) -> Result<json::JSONData<'b>> {
        if let Ok(_) = self.eat_str("true") {
            Ok(json::JSONData::Bool(true))
        } else if let Ok(_) = self.eat_str("false") {
            Ok(json::JSONData::Bool(false))
        } else {
            self.fail("parse::boolean")
        }
    }

    fn number(&mut self) -> Result<json::JSONData<'b>> {
        let ptr_start = self.current_byte_as_ptr();
        let idx_start = self.index;

        // eat through valid bytes, skip initial ws
        let mut next = self.walk(true)?;
        while match next {
            b'0'...b'9' | b'-' | b'.' | b'e' | b'E' => {
                self.ate(next);
                next = self.walk(false).unwrap_or(b'\x00');
                true
            }
            _ => false,
        } {}

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
        let next = self.walk(true)?;
        // lookahead
        match next {
            b'[' => self.array().map(json::JSONData::Array),
            b'{' => self.object().map(json::JSONData::Object),
            b'0'...b'9' | b'-' => self.number(),
            b't' | b'f' => self.boolean(),
            b'n' => self.null(),
            b'"' => self.text(),
            _ => self.fail("parse::value lookahead failed"),
        }
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
