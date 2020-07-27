#![forbid(unsafe_code)]
#![forbid(bare_trait_objects)]
/*!
 *
 *
 */

use std::borrow::Cow;
use std::io::{ErrorKind, Read};

pub struct Parser<R> {
    src: R,
}

impl<R: Read> Parser<R> {
    pub fn new(r: R) -> Self {
        Self { src: r }
    }

    pub fn next(&mut self) -> Option<Json> {
        let j = match self.next_byte()? {
            b'0'..=b'9' | b'-' => Json::Null,
            b'[' => Json::Array(ParseArray { base: self }),
            b'{' => Json::Object(ParseObject { base: self }),
            _ => Json::Null,
        };

        Some(j)
    }
}

/// This trait exists to allow `ParseArray` and `ParseObject` to
/// not depend on the original `R: Read` from the base `Parser`
trait Parse {
    fn next_byte(&mut self) -> Option<u8>;
}

impl<R: Read> Parse for Parser<R> {
    fn next_byte(&mut self) -> Option<u8> {
        let mut b = [0];
        match self.src.read_exact(&mut b) {
            Ok(()) => Some(b[0]),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => None,
            Err(e) => panic!("error reading: {:?}", e),
        }
    }
}

pub struct Number(f64);
pub struct ParseArray<'a> {
    base: &'a mut dyn Parse,
}

impl<'a> ParseArray<'a> {
    pub fn next(&mut self) -> Option<Json> {
        Some(Json::Null)
    }
}

pub struct ParseObject<'a> {
    base: &'a mut dyn Parse,
}

impl<'a> ParseObject<'a> {
    pub fn next(&mut self) -> Option<KeyVal> {
        Some(KeyVal { base: self.base })
    }

    //pub fn find_key(self) -> Option<Json> { }
}

/// Reads a key and/or value pair of an object.
///
/// They key and the value may be read independently, and either may be ignored.
///
/// For example, it's possible the only read the key, and ignore the value,
/// which will be skipped efficiently.
pub struct KeyVal<'a> {
    base: &'a mut dyn Parse,
}

impl<'a> KeyVal<'a> {
    pub fn key(&mut self) -> ParseString {
        ParseString { base: self.base }
    }

    pub fn value(&mut self) -> Json {
        Json::Null
    }
}

/// Reads a string. Reading can be done as a whole string,
/// (or even str when no escape sequences are present),
/// or char-by-char if the string is expected to be very large.
pub struct ParseString<'a> {
    base: &'a mut dyn Parse,
}

impl<'a> ParseString<'a> {
    pub fn read_cow(self) -> Cow<'a, str> {
        Cow::Borrowed("")
    }

    pub fn read_owned(self) -> String {
        "".into()
    }

    pub fn read_chars(self) -> Chars<'a> {
        Chars { base: self.base }
    }
}

pub struct Chars<'a> {
    base: &'a mut dyn Parse,
}

impl<'a> Iterator for Chars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/// Represents a json value (null, bool, numbers),
/// or holds a parser that yields a larger value (string, array, object)
pub enum Json<'a> {
    Null,
    Bool(bool),
    Number(Number),
    String(ParseString<'a>),
    Array(ParseArray<'a>),
    Object(ParseObject<'a>),
}
