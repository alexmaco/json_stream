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

type YielfFn = for<'r> fn(&'r mut (dyn Parse + 'r), u8) -> Json<'r>;

//fn next_any_item(b: u8) -> Option<for <'r> fn(&'r mut (dyn Parse + 'r), u8) -> Json<'r>> {
fn next_any_item(b: u8) -> Option<YielfFn> {
    if b.is_ascii_whitespace() {
        return None;
    }

    Some(match b {
        b'0'..=b'9' | b'-' => |p, b| parse_number(p, b),
        b'n' => |p, _| parse_ident(p, b"ull", Json::Null),
        b't' => |p, _| parse_ident(p, b"rue", Json::Bool(true)),
        b'f' => |p, _| parse_ident(p, b"alse", Json::Bool(false)),
        b'[' => |p, _| Json::Array(ParseArray::new(p)),
        b'{' => |p, _| Json::Object(ParseObject { parse: p }),
        b'"' => |p, _| Json::String(ParseString::new(p)),
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

pub struct ParseArray<'a> {
    parse: &'a mut dyn Parse,
    ended: bool,
}

use std::any::type_name;
use std::fmt::{self, Debug, Formatter};
impl Debug for ParseString<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{} for Parser@{:p}>",
            type_name::<Self>(),
            self.parse.as_ref().unwrap()
        )
    }
}
impl Debug for ParseArray<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} for Parser@{:p}>", type_name::<Self>(), self.parse)
    }
}
impl Debug for ParseObject<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} for Parser@{:p}>", type_name::<Self>(), self.parse)
    }
}

impl<'a> ParseArray<'a> {
    fn new(parse: &'a mut (dyn Parse + 'a)) -> Self {
        Self {
            parse: parse,
            ended: false,
        }
    }

    pub fn next(&mut self) -> Option<Json> {
        while !self.ended {
            let b = self.parse.next_byte()?;
            match b {
                b']' => {
                    self.ended = true;
                    break;
                }
                b',' => continue,
                _ => match next_any_item(b) {
                    Some(f) => return Some(f(self.parse, b)),
                    _ => continue,
                },
            }
        }
        None
    }
}

impl Drop for ParseArray<'_> {
    fn drop(&mut self) {
        if !self.ended {
            while self.next().is_some() {}
        }
    }
}

//fn skip_array(parse: &mut dyn Parse) { todo!("implement efficient skipping") }

pub struct ParseObject<'a> {
    parse: &'a mut dyn Parse,
}

impl<'a> ParseObject<'a> {
    pub fn next(&mut self) -> Option<KeyVal> {
        loop {
            let b = self.parse.peek_byte()?;
            match b {
                _ if b.is_ascii_whitespace() || b == b',' => {
                    self.parse.next_byte();
                    continue;
                }
                b'}' => {
                    self.parse.next_byte();
                    return None;
                }
                b'"' => {
                    self.parse.next_byte();
                    break;
                }
                _ => panic!("unhandled char '{}' in object", char::from(b)),
            }
        }
        Some(KeyVal::new(self.parse))
    }
}

/// Reads a key and/or value pair of an object.
///
/// They key and the value may be read independently, and either may be ignored.
///
/// For example, it's possible the only read the key, and ignore the value,
/// which will be skipped efficiently.
pub struct KeyVal<'a> {
    parse: Option<&'a mut dyn Parse>,
    key_consumed: bool,
    val_consumed: bool,
}

impl<'a> KeyVal<'a> {
    fn new(parse: &'a mut (dyn Parse + 'a)) -> Self {
        Self {
            parse: Some(parse),
            key_consumed: false,
            val_consumed: false,
        }
    }

    /// Obtains a [`Json`] for this object key.
    /// Panics if called more than once.
    pub fn key(&mut self) -> ParseString {
        assert_eq!(self.key_consumed, false);
        self.key_consumed = true;
        ParseString::new(*self.parse.as_mut().unwrap())
    }

    /// Obtains a [`Json`] for this object value.
    /// Skips and discards the key if it was not already retrieved.
    pub fn value(mut self) -> Json<'a> {
        self.val_consumed = true;
        let parse = self.parse.take().unwrap();
        let (f, b) = read_value(parse, self.key_consumed);
        f(parse, b)
    }
}

impl<'a> Drop for KeyVal<'a> {
    fn drop(&mut self) {
        if self.val_consumed {
            return;
        }
        let parse = self.parse.take();
        if let Some(parse) = parse {
            let (f, b) = read_value(parse, self.key_consumed);
            drop(f(parse, b))
        }
    }
}

fn read_value<'a>(parse: &'a mut (dyn Parse + 'a), key_consumed: bool) -> (YielfFn, u8) {
    if !key_consumed {
        drop(ParseString::new(parse)) // skip the key string
    }

    loop {
        let b = parse.next_byte().unwrap();
        match b {
            _ if b.is_ascii_whitespace() => continue,
            b':' => break,
            _ => panic!("unhandled char '{}' in object", char::from(b)),
        }
    }
    loop {
        let b = parse.next_byte().unwrap();
        if let Some(f) = next_any_item(b) {
            return (f, b); //f(parse, b);
        }
    }
}

/// Reads a string. Reading can be done as a whole string,
/// or char-by-char if the string is expected to be very large.
pub struct ParseString<'a> {
    parse: Option<&'a mut dyn Parse>,
}

impl<'a> ParseString<'a> {
    fn new(parse: &'a mut (dyn Parse + 'a)) -> Self {
        Self { parse: Some(parse) }
    }

    /// Parses the entire JSON string into a new [`String`]
    pub fn read_owned(self) -> String {
        let mut buf = String::new();
        self.read_into(&mut buf);
        buf
    }

    /// Parses the entire string into the supplied [`String`].
    /// This is used to avoid allocating new string,
    /// or to preallocate a buffer when the client code assumes a certain length.
    pub fn read_into(mut self, buf: &mut String) {
        let base = self.parse.take().unwrap();
        loop {
            let c = base.next_byte().unwrap();
            if c == b'"' {
                break;
            }

            buf.push(c.into());
        }
    }

    /// Parses this JSON string one [`char`] at a time,
    /// instead of the entire string.
    pub fn read_chars(mut self) -> ParseChars<'a> {
        ParseChars::new(self.parse.take().unwrap())
    }
}

impl Drop for ParseString<'_> {
    fn drop(&mut self) {
        if let Some(p) = self.parse.as_mut() {
            skip_string(*p)
        }
    }
}

fn skip_string(parse: &mut dyn Parse) {
    let mut escape = false;
    loop {
        let b = parse.next_byte().unwrap();
        match b {
            b'\\' if !escape => escape = true,
            b'"' if !escape => return,
            _ => escape = false,
        }
    }
}

pub struct ParseChars<'a> {
    parse: &'a mut dyn Parse,
}

impl<'a> ParseChars<'a> {
    fn new(parse: &'a mut (dyn Parse + 'a)) -> Self {
        Self { parse: parse }
    }
}

impl<'a> Iterator for ParseChars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let mut escape = false;
        loop {
            match self.parse.next_byte().unwrap() {
                b'\\' if !escape => {
                    escape = true;
                    continue;
                }
                b'"' if !escape => return None,
                c => return Some(c.into()),
            }
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

impl<'a> Json<'a> {
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

    pub fn is_string(&self) -> bool {
        match self {
            Self::String(_) => true,
            _ => false,
        }
    }

    pub fn as_string(self) -> Option<ParseString<'a>> {
        match self {
            Self::String(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Self::Array(_) => true,
            _ => false,
        }
    }

    pub fn as_array(self) -> Option<ParseArray<'a>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            Self::Object(_) => true,
            _ => false,
        }
    }

    pub fn as_object(self) -> Option<ParseObject<'a>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }
}

pub enum Error {
    /// an unquoted string other than "null", "true", or "false" was encountered and skipped
    InvalidIdentifier,
}
