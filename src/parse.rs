use crate::json::JSONValue;

use std::collections::HashMap;
use std::fmt;
use std::slice;
use std::str;

use lexical_core;

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
    bytes: &'a [u8],
    text: &'a str,
    index: usize, // index in byte sequence _bytes_
}

#[derive(Debug)]
pub enum ParseError {
    EndOfStream,
    UnexpectedByte { token: char },
    IllegalByte,
    FailedMatch,
    ExpectedBoolean,
    ExpectedNumber,
    ExpectedValue,
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
            bytes: text.as_bytes(),
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
            _ => Err(ParseError::EndOfStream),
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

    #[inline(always)]
    fn accept(&mut self) {
        self.index += 1;
    }

    #[inline(always)]
    fn accept_n(&mut self, n: usize) {
        self.index += n;
    }

    /// Consumes expected token
    fn eat(&mut self, token: u8) -> self::Result<()> {
        let next = self.current_byte()?;

        if next == token {
            self.accept();

            Ok(())
        } else {
            Err(ParseError::FailedMatch)
        }
    }

    /// Consumes expected string
    fn eat_str(&mut self, match_str: &'static str) -> self::Result<&str> {
        let match_bytes = match_str.as_bytes();

        if match_bytes == &self.bytes[self.index..self.index + match_bytes.len()] {
            self.accept_n(match_bytes.len());
            Ok(match_str)
        } else {
            Err(ParseError::FailedMatch)
        }
    }

    /// Consumes all bytes up intil an expected end byte
    fn eat_until(&mut self, token: u8) -> self::Result<&'b str> {
        let ptr_start = self.current_byte_as_ptr();
        let idx_start = self.index;

        loop {
            let b = self.current_byte()?;

            if b == token {
                break;
            }

            // illegal byte, fail
            if !ALLOWED[b as usize] {
                return Err(ParseError::IllegalByte);
            }

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

    /// Consumes an object
    fn object(&mut self) -> self::Result<JSONValue<'b>> {
        self.skip_ctrl_bytes();
        self.eat(b'{')?;

        let fields = self.fields()?;

        self.skip_ctrl_bytes();
        self.eat(b'}')?;
        Ok(JSONValue::Object(fields))
    }

    /// Consumes object fields
    fn fields(&mut self) -> self::Result<HashMap<&'b str, JSONValue<'b>>> {
        let mut hashmap = HashMap::<&str, JSONValue<'b>>::new();

        while let Ok((id, value)) = self.field() {
            let _ = hashmap.insert(id, value);
        }

        Ok(hashmap)
    }

    /// Consumes an identifier and a value
    fn field(&mut self) -> Result<(&'b str, JSONValue<'b>)> {
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
    fn array(&mut self) -> self::Result<JSONValue<'b>> {
        self.skip_ctrl_bytes();
        self.eat(b'[')?;

        let values = self.values()?;

        self.skip_ctrl_bytes();
        self.eat(b']')?;

        Ok(JSONValue::Array(values))
    }

    /// Consumes comma separated values
    fn values(&mut self) -> self::Result<Vec<JSONValue<'b>>> {
        let mut vals = Vec::<JSONValue<'b>>::new();

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
    fn null(&mut self) -> Result<JSONValue<'b>> {
        self.eat_str("null")?;
        Ok(JSONValue::Null)
    }

    /// Consumes a Text value
    fn text(&mut self) -> Result<JSONValue<'b>> {
        let s = self.string()?;
        Ok(JSONValue::Text(s))
    }

    /// Consumes a Boolean value
    fn boolean(&mut self) -> Result<JSONValue<'b>> {
        if let Ok(_) = self.eat_str("true") {
            Ok(JSONValue::Bool(true))
        } else if let Ok(_) = self.eat_str("false") {
            Ok(JSONValue::Bool(false))
        } else {
            Err(ParseError::ExpectedBoolean)
        }
    }

    /// Consumes a f64 Number value
    fn number(&mut self) -> Result<JSONValue<'b>> {
        let idx_start = self.index;

        // eat through valid bytes
        while match self.current_byte().unwrap_or(b'\0') {
            b'0'...b'9' | b'-' | b'.' | b'e' | b'E' => true,
            _ => false,
        } {
            self.accept();
        }

        // checked parse
        let res = lexical_core::try_atof64_slice(&self.bytes[idx_start..self.index]);

        if res.error.code == lexical_core::ErrorCode::Success {
            Ok(JSONValue::Number(res.value))
        } else {
            Err(ParseError::ExpectedNumber)
        }
    }

    /// Consumes a valid value
    pub fn value(&mut self) -> Result<JSONValue<'b>> {
        self.skip_ctrl_bytes();

        let next = self.current_byte()?;

        // lookahead
        match next {
            b'[' => self.array(),
            b'{' => self.object(),
            b'0'...b'9' | b'-' => self.number(),
            b't' | b'f' => self.boolean(),
            b'n' => self.null(),
            b'"' => self.text(),
            _ => Err(ParseError::ExpectedValue),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::json::JSONValue;
    use crate::parse;
    use std::collections::HashMap;

    #[test]
    fn parse_text_and_boolean() {
        let mut obj = HashMap::<&str, JSONValue>::new();
        obj.insert("myBool", JSONValue::Bool(true));
        obj.insert("myString", JSONValue::Text("SomeString"));

        let txt = r#"{ "myString": "SomeString", "myBool":  true }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), JSONValue::Object(obj));
    }

    #[test]
    fn parse_text_and_boolean_trailing_comma() {
        let mut obj = HashMap::<&str, JSONValue>::new();
        obj.insert("myBool", JSONValue::Bool(true));
        obj.insert("myString", JSONValue::Text("SomeString"));

        let txt = r#"{ "myString": "SomeString", "myBool":  true, }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), JSONValue::Object(obj));
    }

    #[test]
    fn parse_nested_object() {
        let mut obj = HashMap::<&str, JSONValue>::new();
        obj.insert("myBool", JSONValue::Bool(true));
        obj.insert("myString", JSONValue::Text("SomeString"));
        let nest = obj.clone();
        obj.insert("myObject", JSONValue::Object(nest));

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

        assert_eq!(res.unwrap(), JSONValue::Object(obj));
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

        assert_eq!(JSONValue::Number(3.14), r1.unwrap());
        assert_eq!(JSONValue::Number(-3.14), r2.unwrap());
        assert_eq!(JSONValue::Number(23.2e-10), r3.unwrap());
        assert_eq!(JSONValue::Number(23.2E10), r4.unwrap());
    }

    #[test]
    fn parse_object() {
        let mut obj = HashMap::<&str, JSONValue>::new();
        obj.insert("myBool", JSONValue::Bool(true));
        obj.insert("myString", JSONValue::Text("SomeString"));

        let mut nest = obj.clone();

        nest.insert("myNumber", JSONValue::Number(33.14));
        nest.insert("myNull", JSONValue::Null);
        nest.insert("myNumber2", JSONValue::Number(-33.14));

        obj.insert("myObject", JSONValue::Object(nest));

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

        assert_eq!(res.unwrap(), JSONValue::Object(obj));
    }

    #[test]
    fn parse_array() {
        let mut map = HashMap::<&str, JSONValue>::new();
        map.insert("myBool", JSONValue::Bool(true));
        map.insert("myString", JSONValue::Text("SomeString"));

        let arr = vec![
            JSONValue::Text("SomeString"),
            JSONValue::Object(map),
            JSONValue::Number(33.14),
        ];

        let txt = r#"

        ["SomeString",
                { "myBool": true, "myString": "SomeString", },

           33.14,]

        "#;

        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.array();

        assert_eq!(res.unwrap(), JSONValue::Array(arr));
    }

    #[test]
    fn parse_text_escaped() {
        let t1 = r#""An\nEscaped\tString""#;

        let mut c1 = parse::ParseContext::new(t1);

        let r1 = c1.text();

        assert_eq!(JSONValue::Text("An\nEscaped\tString"), r1.unwrap());
    }
}
