use crate::json::JSONValue;

use std::collections::HashMap;
use std::fmt;
use std::slice;
use std::str;

use crate::json;

use lexical_core;

//
// Borrowed from: https://github.com/maciejhirsz/json-rust/blob/master/src/parser.rs#L158
//
// Look up table that marks which characters are allowed in their raw
// form in a string.
/*
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
*/

#[derive(Debug)]
pub struct ParseContext<'a> {
    bytes: &'a [u8],
    text: &'a str,
    // storage used for escaped strings and unicode parsing
    buffers: Vec<Vec<u8>>,
    // index in byte sequence _bytes_
    index: usize,
}

#[derive(Debug)]
pub enum Error {
    EndOfStream,
    UnexpectedCharacter {
        line: usize,
        col: usize,
        token: char,
    },
    InvalidJSON,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

type Result<T> = std::result::Result<T, Error>;

impl<'a, 'b: 'a> ParseContext<'a> {
    pub fn new(text: &'b str) -> ParseContext {
        ParseContext {
            bytes: text.as_bytes(),
            text,
            buffers: Vec::new(),
            index: 0,
        }
    }

    pub fn parse(&mut self) -> Result<JSONValue> {
        // valid json starts with object or array
        match self.value() {
            o @ Ok(JSONValue::Object(_)) => o,
            a @ Ok(JSONValue::Array(_)) => a,
            _ => Err(Error::InvalidJSON),
        }
    }

    fn fail<T>(&self) -> Result<T> {
        #[cfg(test)]
        {
            let s = unsafe { str::from_utf8_unchecked(&self.bytes[0..self.index]) };
            println!("{}", s);
        }

        let mut line = 0;
        let mut col = 0;

        Err(Error::UnexpectedCharacter {
            line,
            col,
            token: ' ',
        })
    }

    /// Returns pointer to current byte index
    fn current_byte_as_ptr(&self) -> *const u8 {
        let p = self.bytes.as_ptr();
        unsafe { p.add(self.index) }
    }

    /// Returns byte at index or EOS
    fn current_byte(&self) -> Result<u8> {
        if self.index < self.bytes.len() {
            Ok(self.bytes[self.index])
        } else {
            Err(Error::EndOfStream)
        }
    }

    /// Skips '\n', '\r', '\t', ' '
    fn skip_control_chars(&mut self) {
        while let Ok(byte) = self.current_byte() {
            match byte {
                b'\n' | b'\r' | b'\t' | b' ' => self.accept(),
                _ => break,
            }
        }
    }

    fn accept(&mut self) {
        self.index += 1;
    }

    fn accept_n(&mut self, n: usize) {
        self.index += n;
    }

    fn skip_comma(&mut self) {
        if let Ok(b',') = self.current_byte() {
            self.accept();
        }
    }

    /// Consumes expected token
    fn eat(&mut self, token: u8) -> Result<()> {
        let next = self.current_byte()?;

        if next == token {
            self.accept();

            Ok(())
        } else {
            self.fail()
        }
    }

    /// Consumes expected string
    fn eat_str(&mut self, match_str: &'static str) -> Result<&str> {
        let match_bytes = match_str.as_bytes();

        if match_bytes == &self.bytes[self.index..self.index + match_bytes.len()] {
            self.accept_n(match_bytes.len());
            Ok(match_str)
        } else {
            self.fail()
        }
    }

    /// Consumes all bytes up intil an expected end byte
    fn eat_until(&mut self, token: u8) -> Result<String> {
        let ptr_start = self.current_byte_as_ptr();
        let idx_start = self.index;

        loop {
            let b = self.current_byte()?;

            if b == token {
                break;
            }

            // hit escape char, start buffered parsing
            if b == b'\\' {
                // bring the initial parsed bytes into the buffer
                let initial_str = unsafe {
                    str::from_utf8_unchecked(slice::from_raw_parts(
                        ptr_start,
                        self.index - idx_start,
                    ))
                };

                return self.eat_buffered_until(initial_str, token);
            } else {
                self.accept();
            }
        }

        let idx_end = self.index;

        unsafe {
            Ok(str::from_utf8_unchecked(slice::from_raw_parts(
                ptr_start,
                idx_end - idx_start,
            )).to_owned())
        }
    }

    fn eat_buffered_until(&mut self, initial_str: &'b str, token: u8) -> Result<String> {
        let mut buffer = Vec::from(initial_str.as_bytes());

        loop {
            let b = self.current_byte()?;

            if b == token {
                break;
            }

            if b == b'\\' {
                self.accept();
                let following_b = self.current_byte()?;

                // escape '"' '\' '/' 'b' 'f' 'n' 'r' 't'
                // TODO: unicode: 'u' hex hex hex hex
                match following_b {
                    b'"' => buffer.push(b'\"'),
                    b'\\' => buffer.push(b'\\'),
                    b'/' => buffer.push(b'/'),
                    b'b' => buffer.push(0x8),
                    b'f' => buffer.push(0xC),
                    b'n' => buffer.push(b'\n'),
                    b'r' => buffer.push(b'\r'),
                    b't' => buffer.push(b'\t'),
                    // unexpected byte following escape
                    _ => return self.fail(),
                }

                self.accept();
            } else {
                buffer.push(b);
                self.accept();
            }
        }

        let buffered_str: &'b str = unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(buffer.as_ptr(), buffer.len()))
        };

        self.buffers.push(buffer);

        Ok(buffered_str.to_owned())
    }

    /// Consumes an object
    fn object(&mut self) -> Result<JSONValue> {
        self.skip_control_chars();
        self.eat(b'{')?;

        self.skip_control_chars();
        let fields = self.object_fields()?;

        self.skip_control_chars();
        self.eat(b'}')?;
        Ok(json::object(fields))
    }

    /// Consumes object fields
    fn object_fields(&mut self) -> Result<HashMap<String, JSONValue>> {
        let b = self.current_byte()?;
        let mut hashmap = HashMap::<String, JSONValue>::new();

        if b == b'}' {
            // empty object
            Ok(hashmap)
        } else {
            loop {
                let (id, value) = self.object_field()?;
                let _ = hashmap.insert(id, value);

                // lookahead, check if end of object
                self.skip_control_chars();
                let b = self.current_byte()?;

                if b == b'}' {
                    break;
                }
            }

            Ok(hashmap)
        }
    }

    /// Consumes an identifier and a value
    fn object_field(&mut self) -> Result<(String, JSONValue)> {
        // 1. parse identifier
        // 2. parse value

        self.skip_control_chars();
        let id = self.string()?;

        self.skip_control_chars();
        self.eat(b':')?;

        let val = self.value()?;

        Ok((id, val))
    }

    /// Consumes an array
    fn array(&mut self) -> Result<JSONValue> {
        self.skip_control_chars();
        self.eat(b'[')?;

        self.skip_control_chars();
        let values = self.array_values()?;

        self.skip_control_chars();
        self.eat(b']')?;

        Ok(json::array(values))
    }

    /// Consumes comma separated values
    fn array_values(&mut self) -> Result<Vec<JSONValue>> {
        let b = self.current_byte()?;
        let mut vals = Vec::<JSONValue>::new();

        if b == b']' {
            // empty array
            Ok(vals)
        } else {
            loop {
                let value = self.value()?;
                vals.push(value);

                // lookahead, check if end of object
                self.skip_control_chars();
                let b = self.current_byte()?;

                if b == b']' {
                    break;
                }
            }

            Ok(vals)
        }
    }

    /// Consumes an enquoted string
    fn string(&mut self) -> Result<String> {
        self.eat(b'"')?;
        let s = self.eat_until(b'"')?;
        self.eat(b'"')?;

        Ok(s)
    }

    /// Consumes a Text value
    fn text(&mut self) -> Result<JSONValue> {
        let s = self.string()?;
        Ok(json::text(s))
    }

    /// Consumes a f64 Number value
    fn number(&mut self) -> Result<JSONValue> {
        let idx_start = self.index;

        // eat through valid bytes
        while let Ok(b) = self.current_byte() {
            match b {
                b'0'...b'9' | b'-' | b'.' | b'e' | b'E' => self.accept(),
                _ => break,
            }
        }

        // checked parse
        let res = lexical_core::try_atof64_slice(&self.bytes[idx_start..self.index]);

        if res.error.code == lexical_core::ErrorCode::Success {
            Ok(json::number(res.value))
        } else {
            self.fail()
        }
    }

    /// Consumes a valid value
    fn value(&mut self) -> Result<JSONValue> {
        self.skip_control_chars();

        let next = self.current_byte()?;

        // lookahead
        let res = match next {
            b'0'...b'9' | b'-' => self.number(),
            b't' => {
                self.eat_str("true")?;
                Ok(json::bool(true))
            }
            b'f' => {
                self.eat_str("false")?;
                Ok(json::bool(false))
            }
            b'n' => {
                self.eat_str("null")?;
                Ok(json::null())
            }
            b'"' => self.text(),
            b'[' => self.array(),
            b'{' => self.object(),
            _ => self.fail(),
        };

        // commas may trail
        self.skip_control_chars();
        self.skip_comma();

        res
    }
}

#[cfg(test)]
mod tests {
    use crate::json::JSONValue;
    use crate::json;
    use crate::parse;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn parse_text_and_boolean() {
        let mut obj = HashMap::<String, JSONValue>::new();
        obj.insert("myBool".to_string(), json::bool(true));
        obj.insert("myString".to_string(), json::text("SomeString"));

        let txt = r#"{ "myString": "SomeString", "myBool":  true }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::object(obj));
    }

    #[test]
    fn parse_text_and_boolean_trailing_comma() {
        let mut obj = HashMap::<String, JSONValue>::new();
        obj.insert("myBool".to_string(), json::bool(true));
        obj.insert("myString".to_string(), json::text("SomeString"));

        let txt = r#"{ "myString": "SomeString", "myBool":  true, }"#;
        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.object();

        assert_eq!(res.unwrap(), json::object(obj));
    }

    #[test]
    fn parse_nested_object() {
        let mut obj = HashMap::<String, JSONValue>::new();
        obj.insert("myBool".to_string(), json::bool(true));
        obj.insert("myString".to_string(), json::text("SomeString"));
        let nest = obj.clone();
        obj.insert("myObject".to_string(), json::object(nest));

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

        assert_eq!(res.unwrap(), json::object(obj));
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

        assert_eq!(json::number(3.14), r1.unwrap());
        assert_eq!(json::number(-3.14), r2.unwrap());
        assert_eq!(json::number(23.2e-10), r3.unwrap());
        assert_eq!(json::number(23.2E10), r4.unwrap());
    }

    #[test]
    fn parse_object() {
        let mut obj = HashMap::<String, JSONValue>::new();
        obj.insert("myBool".to_string(), json::bool(true));
        obj.insert("myString".to_string(), json::text("SomeString"));

        let mut nest = obj.clone();

        nest.insert("myNumber".to_string(), json::number(33.14));
        nest.insert("myNull".to_string(), json::null());
        nest.insert("myNumber2".to_string(), json::number(-33.14));

        obj.insert("myObject".to_string(), json::object(nest));

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

        assert_eq!(res.unwrap(), json::object(obj));
    }

    #[test]
    fn parse_array() {
        let mut map = HashMap::<String, JSONValue>::new();
        map.insert("myBool".to_string(), json::bool(true));
        map.insert("myString".to_string(), json::text("SomeString"));

        let arr = vec![
            json::text("SomeString"),
            json::object(map),
            json::number(33.14),
        ];

        let txt = r#"

        ["SomeString",
                { "myBool": true, "myString": "SomeString", },

           33.14,]

        "#;

        let mut ctx = parse::ParseContext::new(txt);
        let res = ctx.array();

        assert_eq!(res.unwrap(), json::array(arr));
    }

    #[test]
    fn parse_text_escaped() {
        let t1 = r#""An\nEscaped\tString""#;

        let mut c1 = parse::ParseContext::new(t1);

        let r1 = c1.text();

        assert_eq!(json::text("An\nEscaped\tString"), r1.unwrap());
    }

    #[test]
    fn parse_consecutive_escaped_strs() {
        let text = r#"
        {
            "myFirs\tt": "Str\\ng",
            "followed": true,
            "by\\": "\tthe\\second",
        }"#;

        let mut ctx = parse::ParseContext::new(text);

        let res = ctx.object();

        let mut map = HashMap::<String, JSONValue>::new();

        map.insert("myFirs\tt".to_string(), json::text("Str\\ng"));
        map.insert("followed".to_string(), json::bool(true));
        map.insert("by\\".to_string(), json::text("\tthe\\second"));

        assert_eq!(json::object(map), res.unwrap());
    }

    fn file_to_str(path: &'static str) -> String {
        let mut f = File::open(path).unwrap();

        let mut txt = String::new();

        let _ = f.read_to_string(&mut txt).unwrap();

        txt
    }

    #[test]
    fn canada_json() {
        let txt = file_to_str("tests/canada.json");

        if let Err(e) = json::from_str(&txt) {
            panic!("Cibola failed with: {}", e);
        }
    }

    #[test]
    fn citm_catalog_json() {
        let txt = file_to_str("tests/citm_catalog.json");

        if let Err(e) = json::from_str(&txt) {
            panic!("Cibola failed with: {}", e);
        }
    }
}
