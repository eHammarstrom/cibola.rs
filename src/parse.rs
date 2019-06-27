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
    last_accept: u8,
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
            token: ctx.current_byte().unwrap_or(b'\0') as char,
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
            last_accept: b'\x00',
            text,
            index: 0,
        }
    }

    /// Returns pointer to current byte index
    fn current_byte_as_ptr(&self) -> *const u8 {
        let p = self.bytes.as_ptr();
        unsafe { p.offset(self.index as isize) }
    }

    /// Returns byte at index or EOS
    fn current_byte(&self) -> Result<u8> {
        match self.bytes.get(self.index) {
            Some(byte) => Ok(*byte),
            _ => Err(ParseError::EOS),
        }
    }

    /// Returns next byte in the sequence or EOS
    fn peek(&self) -> Result<u8> {
        match self.bytes.get(self.index + 1) {
            Some(byte) => Ok(*byte),
            _ => Err(ParseError::EOS),
        }
    }

    /// Skips '\n', '\r', '\t', ' '
    fn skip_ctrl_bytes(&mut self) {
        while let Ok(byte) = self.current_byte() {
            match byte {
                b'\n' | b'\r' | b'\t' | b' ' => self.accept(),
                _ => break,
            }
        }
    }

    fn accept(&mut self) {
        self.index += 1;
        self.last_accept = self.bytes[self.index - 1];
    }

    fn accept_n(&mut self, n: usize) {
        self.index += n;
        self.last_accept = self.bytes[self.index - 1];
    }

    /// Consumes expected token
    fn eat(&mut self, token: u8) -> self::Result<()> {
        let next = self.current_byte()?;

        if next == token {
            self.accept();

            Ok(())
        } else {
            self.fail("parse::eat")
        }
    }

    /// Consumes expected string
    fn eat_str(&mut self, match_str: &'static str) -> self::Result<&str> {
        let match_bytes = match_str.as_bytes();

        if match_bytes == &self.bytes[self.index..self.index + match_bytes.len()] {
            self.accept_n(match_bytes.len());
            Ok(match_str)
        } else {
            self.fail("parse::eat_str")
        }
    }

    /// Consumes all bytes up intil an expected end byte
    fn eat_until(&mut self, token: u8) -> self::Result<&'b str> {
        let ptr_start = self.current_byte_as_ptr();
        let idx_start = self.index;

        while self.current_byte()? != token {
            self.accept();
        }

        let idx_end = self.index;

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

    /// Consumes an object
    pub fn object(&mut self) -> self::Result<json::Object<'b>> {
        self.skip_ctrl_bytes();
        self.eat(b'{')?;

        let fields = self.fields()?;

        self.skip_ctrl_bytes();
        self.eat(b'}')?;
        Ok(json::Object(fields))
    }

    /// Consumes object fields
    fn fields(&mut self) -> self::Result<HashMap<&'b str, json::JSONData<'b>>> {
        let mut hashmap = HashMap::<&str, json::JSONData<'b>>::new();

        while let Ok((id, value)) = self.field() {
            let _ = hashmap.insert(id, value);
        }

        Ok(hashmap)
    }

    /// Consumes an identifier and a value
    fn field(&mut self) -> Result<(&'b str, json::JSONData<'b>)> {
        // 1. parse identifier
        // 2. parse value

        self.skip_ctrl_bytes();
        let id = self.string()?;

        self.skip_ctrl_bytes();
        self.eat(b':')?;

        let val = self.value()?;

        // commas may trail
        self.skip_ctrl_bytes();
        let _ = self.eat(b',');

        Ok((id, val))
    }

    /// Consumes an array
    pub fn array(&mut self) -> self::Result<json::Array<'b>> {
        self.skip_ctrl_bytes();
        self.eat(b'[')?;

        let values = self.values()?;

        self.skip_ctrl_bytes();
        self.eat(b']')?;

        Ok(json::Array(values))
    }

    /// Consumes comma separated values
    fn values(&mut self) -> self::Result<Vec<json::JSONData<'b>>> {
        let mut vals = Vec::<json::JSONData<'b>>::new();

        while let Ok(v) = self.value() {
            let _ = vals.push(v);
            // commas may trail
            self.skip_ctrl_bytes();
            let _ = self.eat(b',');
        }

        Ok(vals)
    }

    /// Consumes an enquoted string
    fn string(&mut self) -> Result<&'b str> {
        self.eat(b'"')?;
        let s = self.eat_until(b'"')?;
        self.eat(b'"')?;

        Ok(s)
    }

    /// Consumes a Null 'value'
    fn null(&mut self) -> Result<json::JSONData<'b>> {
        self.eat_str("null")?;
        Ok(json::JSONData::Null)
    }

    /// Consumes a Text value
    fn text(&mut self) -> Result<json::JSONData<'b>> {
        let s = self.string()?;
        Ok(json::JSONData::Text(s))
    }

    /// Consumes a Boolean value
    fn boolean(&mut self) -> Result<json::JSONData<'b>> {
        if let Ok(_) = self.eat_str("true") {
            Ok(json::JSONData::Bool(true))
        } else if let Ok(_) = self.eat_str("false") {
            Ok(json::JSONData::Bool(false))
        } else {
            self.fail("parse::boolean")
        }
    }

    /// Consumes a f64 Number value
    fn number(&mut self) -> Result<json::JSONData<'b>> {
        let idx_start = self.index;

        // eat through valid bytes
        while match self.current_byte().unwrap_or(b'\0') {
            b'0'...b'9' | b'-' | b'.' | b'e' | b'E' => true,
            _ => false,
        } {
            self.accept();
        }

        let num = unsafe { str::from_utf8_unchecked(&self.bytes[idx_start..self.index]) };

        f64::from_str(&num)
            .map(json::JSONData::Number)
            .or(self.fail("parse::number")) // should be folded
    }

    /// Consumes a valid value
    fn value(&mut self) -> Result<json::JSONData<'b>> {
        self.skip_ctrl_bytes();

        let next = self.current_byte()?;

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

    #[test]
    fn parse_text_escaped() {
        let t1 = r#""An\nEscaped\tString""#;

        let mut c1 = parse::ParseContext::new(t1);

        let r1 = c1.text();

        assert_eq!(json::JSONData::Text("An\nEscaped\tString"), r1.unwrap());
    }
}
