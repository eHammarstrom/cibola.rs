use crate::json;
use std::fmt;

pub struct ParseContext<'a> {
    lineno: u32,
    col: u32,
    nom: Vec<char>,
    text: &'a str,
}

enum ParseError {
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
        let ParseContext {
            lineno, col, nom, ..
        } = ctx;

        let token = nom.last().get_or_insert(&'\0');

        ParseError::UnexpectedToken {
            lineno: *lineno,
            col: *col,
            token: **token,
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
    pub fn object(&mut self) -> self::Result<json::Object> {
        let txt_iter = self.text.chars().peekable();
        let peek = txt_iter.peek().ok_or(ParseError::EOS)?;

        self.nom.push(*peek);

        let fields: Vec<json::JSONField> = match *peek {
            '{' => self.fields(),
            _ => Err(ParseError::unexpected_token(
                &self,
                "failed to parse object",
            )),
        }?;

        Ok(json::Object(fields))
    }

    pub fn fields(&mut self) -> self::Result<Vec<json::JSONField>> {
        Ok(vec![])
    }

    pub fn field(&mut self) -> Result<json::JSONField> {
        Ok(json::JSONField {
            identifier: "lol".to_string(),
            data: json::JSONData::Null,
        })
    }
}
