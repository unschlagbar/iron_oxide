//! A small JSON value type with a parser and a compact writer.
//!
//! Numbers keep their source text, so integers past 2^53 and decimal
//! amounts survive a round trip without going through `f64`.

use std::fmt::{self, Display, Write};
use std::ops::Index;

/// Deepest nesting the parser accepts, so hostile input can't overflow the stack.
const MAX_DEPTH: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// The number as written, already checked against the JSON grammar.
    Number(String),
    String(String),
    Array(Vec<Json>),
    /// Members in document order.
    Object(Vec<(String, Json)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub message: &'static str,
    /// Byte offset into the input.
    pub at: usize,
}

impl Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "json: {} at byte {}", self.message, self.at)
    }
}

impl std::error::Error for JsonError {}

static NULL: Json = Json::Null;

impl Json {
    pub fn parse(input: &str) -> Result<Self, JsonError> {
        let mut parser = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        let value = parser.value(0)?;
        parser.skip_whitespace();
        if parser.pos != parser.bytes.len() {
            return Err(parser.error("trailing characters"));
        }
        Ok(value)
    }

    /// An object from `(key, value)` pairs.
    pub fn object<K: Into<String>>(members: impl IntoIterator<Item = (K, Json)>) -> Self {
        Self::Object(members.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn array(items: impl IntoIterator<Item = Json>) -> Self {
        Self::Array(items.into_iter().collect())
    }

    /// The first member named `key`, if this is an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Sets `key`, replacing an existing member. Does nothing on non-objects.
    pub fn set(&mut self, key: impl Into<String>, value: Json) {
        let Self::Object(members) = self else {
            return;
        };
        let key = key.into();
        match members.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => *v = value,
            None => members.push((key, value)),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// The number's source text.
    pub fn as_number(&self) -> Option<&str> {
        match self {
            Self::Number(n) => Some(n),
            _ => None,
        }
    }

    /// `None` for fractions, exponents and values out of range.
    pub fn as_i64(&self) -> Option<i64> {
        self.as_number()?.parse().ok()
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_number()?.parse().ok()
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_number()?.parse().ok()
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Self::Object(members) => Some(members),
            _ => None,
        }
    }
}

/// Missing keys and indices give `Json::Null`, so lookups chain.
impl Index<&str> for Json {
    type Output = Json;

    fn index(&self, key: &str) -> &Json {
        self.get(key).unwrap_or(&NULL)
    }
}

impl Index<usize> for Json {
    type Output = Json;

    fn index(&self, i: usize) -> &Json {
        self.as_array().and_then(|a| a.get(i)).unwrap_or(&NULL)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

macro_rules! from_int {
    ($($t:ty),*) => {$(
        impl From<$t> for Json {
            fn from(n: $t) -> Self {
                Self::Number(n.to_string())
            }
        }
    )*};
}

from_int!(i8, i16, i32, i64, u8, u16, u32, u64, usize, isize);

/// Non-finite values have no JSON form and become `null`.
impl From<f64> for Json {
    fn from(n: f64) -> Self {
        if n.is_finite() {
            Self::Number(format!("{n:?}"))
        } else {
            Self::Null
        }
    }
}

impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}

/// Compact output with no whitespace.
impl Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => f.write_str(n),
            Self::String(s) => write_string(f, s),
            Self::Array(items) => {
                f.write_char('[')?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    item.fmt(f)?;
                }
                f.write_char(']')
            }
            Self::Object(members) => {
                f.write_char('{')?;
                for (i, (key, value)) in members.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write_string(f, key)?;
                    f.write_char(':')?;
                    value.fmt(f)?;
                }
                f.write_char('}')
            }
        }
    }
}

fn write_string(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, message: &'static str) -> JsonError {
        JsonError {
            message,
            at: self.pos,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, b: u8, message: &'static str) -> Result<(), JsonError> {
        if self.peek() != Some(b) {
            return Err(self.error(message));
        }
        self.pos += 1;
        Ok(())
    }

    fn literal(&mut self, word: &[u8], value: Json) -> Result<Json, JsonError> {
        if !self.bytes[self.pos..].starts_with(word) {
            return Err(self.error("unknown literal"));
        }
        self.pos += word.len();
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.skip_whitespace();
        match self.peek() {
            None => Err(self.error("unexpected end of input")),
            Some(b'n') => self.literal(b"null", Json::Null),
            Some(b't') => self.literal(b"true", Json::Bool(true)),
            Some(b'f') => self.literal(b"false", Json::Bool(false)),
            Some(b'"') => self.string().map(Json::String),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.pos += 1;
        let mut members = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a key"));
            }
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(b':', "expected ':'")?;
            members.push((key, self.value(depth + 1)?));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(members));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.error("expected a digit")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if self.digits() == 0 {
                return Err(self.error("expected a digit after '.'"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(self.error("expected a digit in the exponent"));
            }
        }
        // Only ASCII was consumed, so this slice is valid UTF-8.
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap();
        Ok(Json::Number(text.to_string()))
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let hex = self
            .bytes
            .get(self.pos..self.pos + 4)
            .ok_or(self.error("truncated \\u escape"))?;
        let mut n = 0;
        for &b in hex {
            let digit = (b as char).to_digit(16).ok_or(self.error("bad \\u escape"))?;
            n = n * 16 + digit;
        }
        self.pos += 4;
        Ok(n)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            // Copy the run up to the next quote, escape or control character.
            let start = self.pos;
            while matches!(self.peek(), Some(b) if b != b'"' && b != b'\\' && b >= 0x20) {
                self.pos += 1;
            }
            // The input is a &str and the run ends on an ASCII byte, so it's valid UTF-8.
            out.push_str(std::str::from_utf8(&self.bytes[start..self.pos]).unwrap());

            match self.peek() {
                None => return Err(self.error("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let escape = self.peek().ok_or(self.error("unterminated string"))?;
                    self.pos += 1;
                    let c = match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.unicode_escape()?,
                        _ => return Err(self.error("unknown escape")),
                    };
                    out.push(c);
                }
                Some(_) => return Err(self.error("control character in string")),
            }
        }
    }

    /// A `\uXXXX` escape, joining a UTF-16 surrogate pair into one char.
    fn unicode_escape(&mut self) -> Result<char, JsonError> {
        let high = self.hex4()?;
        let code = match high {
            0xd800..=0xdbff => {
                if !self.bytes[self.pos..].starts_with(b"\\u") {
                    return Err(self.error("unpaired surrogate"));
                }
                self.pos += 2;
                let low = self.hex4()?;
                if !(0xdc00..=0xdfff).contains(&low) {
                    return Err(self.error("unpaired surrogate"));
                }
                0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
            }
            0xdc00..=0xdfff => return Err(self.error("unpaired surrogate")),
            code => code,
        };
        char::from_u32(code).ok_or(self.error("bad \\u escape"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_values() {
        let json = Json::parse(r#" {"a": [1, -2.5e3, true, null], "b": {"c": "d"}} "#).unwrap();
        assert_eq!(json["a"][0].as_i64(), Some(1));
        assert_eq!(json["a"][1].as_f64(), Some(-2500.0));
        assert_eq!(json["a"][2].as_bool(), Some(true));
        assert!(json["a"][3].is_null());
        assert_eq!(json["b"]["c"].as_str(), Some("d"));
        assert!(json["missing"]["deeper"].is_null());
    }

    #[test]
    fn keeps_large_integers_exact() {
        let json = Json::parse("18446744073709551615").unwrap();
        assert_eq!(json.as_u64(), Some(u64::MAX));
        assert_eq!(json.to_string(), "18446744073709551615");
    }

    #[test]
    fn decodes_escapes_and_surrogate_pairs() {
        let json = Json::parse(r#""a\"b\\c\/\n\u00e4\ud83d\ude00""#).unwrap();
        assert_eq!(json.as_str(), Some("a\"b\\c/\nä😀"));
    }

    #[test]
    fn round_trips_through_display() {
        let value = Json::object([
            ("text", Json::from("quote \" tab \t bell \u{7} ü")),
            ("n", Json::from(42u32)),
            ("list", Json::array([Json::Null, Json::from(false)])),
        ]);
        let text = value.to_string();
        assert_eq!(
            text,
            r#"{"text":"quote \" tab \t bell \u0007 ü","n":42,"list":[null,false]}"#
        );
        assert_eq!(Json::parse(&text).unwrap(), value);
    }

    #[test]
    fn set_replaces_or_appends() {
        let mut json = Json::object([("a", Json::from(1u8))]);
        json.set("a", Json::from(2u8));
        json.set("b", Json::from(3u8));
        assert_eq!(json.to_string(), r#"{"a":2,"b":3}"#);
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "", "[1,]", "{\"a\" 1}", "01", "1.", "-", "1e", "\"abc", "\"\\x\"", "\"\u{1}\"",
            "tru", "[1] 2", "\"\\ud800\"", "\"\\udc00\"", "{1:2}",
        ] {
            assert!(Json::parse(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn limits_nesting_depth() {
        let deep = "[".repeat(MAX_DEPTH + 2) + &"]".repeat(MAX_DEPTH + 2);
        assert_eq!(Json::parse(&deep).unwrap_err().message, "nested too deeply");
        let ok = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert!(Json::parse(&ok).is_ok());
    }
}
