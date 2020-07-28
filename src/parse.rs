//! # Parse
//!
//! This module provides a way to lazily parse JSON data.
//! A [`Parser`] can read from any object implementing [`Read`], and will yield
//! [`Json`] values in sequence like an iterator. Fixed-size items are parsed as values directly,
//! but for strings, arrays and objects subparsers are returned instead.
//! The caller can then invoke these subparsers to effectively parse the corresponsing item content.
//!
//!
//! ## Skipping
//!
//! When a [`ParseString`], [`ParseArray`], [`ParseObject`], or [`KeyVal`] is dropped,
//! it marks that item for skipping. When the next JSON item is requested via a call to `fn next`,
//! the item beyond the skipped one is returned.
//!
//! This allows efficient skipping of uninteresting items.

use std::io::{self, ErrorKind, Read};
use std::iter::Peekable;

/// Reads bytes from a [`Read`], parses them as [`Json`], and returns a stream of values or sub-parsers via `fn next()`
pub struct Parser<R: Read> {
    src: Peekable<io::Bytes<R>>,
}

impl<R: Read> Parser<R> {
    /// Constructs a new Parser that will read from the provided object.
    pub fn new(r: R) -> Self {
        Self {
            src: r.bytes().peekable(),
        }
    }

    /// Returns the next JSON item.
    /// A Parser will read any number of whitespace-separated JSON items and return them in order.
    /// Returns None when the input is exhausted.
    pub fn next(&mut self) -> Option<Json> {
        loop {
            let b = self.next_byte()?;
            break match next_any_item(b) {
                Some(f) => Some(f(self, b)),
                _ => continue,
            };
        }
    }
}

/// This trait exists to allow `ParseArray` and `ParseObject` to
/// not depend on the original `R: Read` from the base `Parser`
trait Parse {
    fn next_byte(&mut self) -> Option<u8>;
    fn peek_byte(&mut self) -> Option<u8>;
}

impl<R: Read> Parse for Parser<R> {
    fn next_byte(&mut self) -> Option<u8> {
        match self.src.next()? {
            Ok(b) => Some(b),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => None,
            Err(e) => panic!("error reading: {:?}", e),
        }
    }
    fn peek_byte(&mut self) -> Option<u8> {
        match self.src.peek()? {
            Ok(b) => Some(*b),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => None,
            Err(e) => panic!("error reading: {:?}", e),
        }
    }
}

fn next_any_item(b: u8) -> Option<fn(&mut dyn Parse, u8) -> Json<'_>> {
    if b.is_ascii_whitespace() {
        return None;
    }

    Some(match b {
        b'0'..=b'9' | b'-' => |p, b| parse_number(p, b),
        b'n' => |p, _| parse_ident(p, b"ull", Json::Null),
        b't' => |p, _| parse_ident(p, b"rue", Json::Bool(true)),
        b'f' => |p, _| parse_ident(p, b"alse", Json::Bool(false)),
        b'[' => |p, _| Json::Array(ParseArray { base: p }),
        b'{' => |p, _| Json::Object(ParseObject { base: p }),
        b'"' => |p, _| Json::String(ParseString { base: p }),
        other => panic!("unhandled {:?}", char::from(other)),
    })
}

fn parse_ident<'a>(parse: &mut dyn Parse, ident: &[u8], res: Json<'a>) -> Json<'a> {
    for b in ident {
        assert_eq!(Some(*b), parse.next_byte());
    }
    res
}

fn parse_number(parse: &mut dyn Parse, byte: u8) -> Json {
    let mut s = String::new();
    s.push(byte.into());
    while let Some(b) = parse.peek_byte() {
        match b {
            b'0'..=b'9' | b'.' | b'e' | b'+' | b'-' => {
                s.push(b.into());
                parse.next_byte();
            }
            _ => break,
        }
    }

    if let Ok(n) = s.parse::<u64>() {
        return Json::Number(Number::from(n));
    }

    if let Ok(n) = s.parse::<i64>() {
        return Json::Number(Number::from(n));
    }

    let n = s.parse::<f64>().unwrap();
    Json::Number(Number::from(n))
}

/// Represents a JSON number (integer or float)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number {
    n: NumRepr,
}

// representation idea lifted from serde_json
#[derive(Debug, Copy, Clone, PartialEq)]
enum NumRepr {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

// from serde_json
macro_rules! impl_from_unsigned {
    ( $($ty:ty),* ) => {
        $(
            impl From<$ty> for Number {
                #[inline]
                fn from(u: $ty) -> Self {
                    let n = { NumRepr::PosInt(u as u64) };
                    Number { n }
                }
            }
        )*
    };
}

// also, from serde_json
macro_rules! impl_from_signed {
    ( $($ty:ty),* ) => {
        $(
            impl From<$ty> for Number {
                #[inline]
                fn from(i: $ty) -> Self {
                    let n = if i < 0 {
                                NumRepr::NegInt(i as i64)
                            } else {
                                NumRepr::PosInt(i as u64)
                            };
                    Number { n }
                }
            }
        )*
    };
}

impl_from_unsigned!(u8, u16, u32, u64, usize);
impl_from_signed!(i8, i16, i32, i64, isize);

impl From<f64> for Number {
    fn from(float: f64) -> Self {
        Number {
            n: NumRepr::Float(float),
        }
    }
}

impl From<f32> for Number {
    fn from(float: f32) -> Self {
        Number {
            n: NumRepr::Float(float.into()),
        }
    }
}

// impl<I> From<I> for Number
// where
//     N: Into<u64>,
// {
//     fn from(n: N) -> Self {
//         Number {
//             n: NumRepr::PosInt(n.into()),
//         }
//     }
// }

pub struct ParseArray<'a> {
    base: &'a mut dyn Parse,
}

use std::any::type_name;
use std::fmt::{self, Debug, Formatter};
impl Debug for ParseString<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} for Parser@{:p}>", type_name::<Self>(), self.base)
    }
}
impl Debug for ParseArray<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} for Parser@{:p}>", type_name::<Self>(), self.base)
    }
}
impl Debug for ParseObject<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} for Parser@{:p}>", type_name::<Self>(), self.base)
    }
}

impl<'a> ParseArray<'a> {
    pub fn next(&mut self) -> Option<Json> {
        loop {
            let b = self.base.next_byte()?;
            match b {
                b']' => return None,
                b',' => continue,
                _ => match next_any_item(b) {
                    Some(f) => return Some(f(self.base, b)),
                    _ => continue,
                },
            }
        }
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
/// or char-by-char if the string is expected to be very large.
pub struct ParseString<'a> {
    base: &'a mut dyn Parse,
}

impl<'a> ParseString<'a> {
    pub fn read_owned(self) -> String {
        let mut buf = String::new();
        loop {
            let c = self.base.next_byte().unwrap();
            if c == b'"' {
                break;
            }

            buf.push(c.into());
        }
        buf
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
        match self.base.next_byte().unwrap() {
            b'"' => None,
            c => Some(c.into()),
        }
    }
}

/// Represents a json value (null, bool, numbers),
/// or holds a parser that yields a larger value (string, array, object)
#[derive(Debug)]
pub enum Json<'a> {
    Null,
    Bool(bool),
    Number(Number),
    String(ParseString<'a>),
    Array(ParseArray<'a>),
    Object(ParseObject<'a>),
}

impl Json<'_> {
    pub fn is_null(&self) -> bool {
        match self {
            Self::Null => true,
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    pub fn as_number(&self) -> Option<Number> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
}

pub enum Error {
    /// an unquoted string other than "null", "true", or "false" was encountered and skipped
    InvalidIdentifier,
}
