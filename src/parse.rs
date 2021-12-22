//! # Parse json
//!
//! This module provides a way to lazily parse JSON data.
//! A [`Parser`] reads from anything implementing [`Read`], and will yield
//! a sequence of [`Json`] values. Fixed-size items are parsed as values directly,
//! but for strings, arrays and objects, subparsers are returned instead.
//! The caller can then invoke these subparsers to actually parse the content of that item.
//!
//!
//! ## Skipping
//!
//! When a [`ParseString`], [`ParseArray`], [`ParseObject`], or [`KeyVal`] is dropped,
//! that item, and everything it contains is skipped. Skipping is done efficiently and lazily,
//! occurring only on the following call to `fn next`, which will return the next Json item
//! on the same level.

use core::convert::TryFrom;
use std::io::{self, ErrorKind, Read};
use std::iter::Peekable;

/// Reads bytes from a [`Read`], parses them as [`Json`], and returns a stream of values or sub-parsers via `fn next()`
pub struct Parser<R: Read> {
    src: Peekable<io::Bytes<R>>,
    skips: Vec<Skip>,
}

type JResult<'a> = std::result::Result<Json<'a>, Error>;

impl<R: Read> Parser<R> {
    /// Constructs a new Parser that will read from the provided object.
    pub fn new(r: R) -> Self {
        Self {
            src: r.bytes().peekable(),
            skips: vec![],
        }
    }

    /// Returns the next JSON item.
    /// A Parser will read any number of whitespace-separated JSON items and return them in order.
    /// Returns None when the input is exhausted.
    pub fn next(&mut self) -> Option<JResult> {
        self.do_skips();
        self.eat_whitespace();
        Some(next_any_item(self.next_byte()?, self))
    }
}

/// This trait exists to allow `ParseArray` and `ParseObject` to
/// not depend on the original `R: Read` from the base `Parser`
trait Parse {
    fn next_byte(&mut self) -> Option<u8>;
    fn peek_byte(&mut self) -> Option<u8>;
    fn eat_until_whitespace(&mut self);
    fn eat_whitespace(&mut self);
    fn add_skip(&mut self, s: Skip);
    fn do_skips(&mut self);
}

#[derive(Debug, Copy, Clone)]
enum Skip {
    Array,
    Object,
    ObjectValue { key_consumed: bool },
    String,
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
    fn eat_until_whitespace(&mut self) {
        loop {
            match self.next_byte() {
                None => break,
                Some(b) => {
                    if b.is_ascii_whitespace() {
                        break;
                    }
                }
            }
        }
    }
    fn eat_whitespace(&mut self) {
        loop {
            match self.peek_byte() {
                None => break,
                Some(b) => {
                    if !b.is_ascii_whitespace() {
                        break;
                    }
                    self.next_byte();
                }
            }
        }
    }
    fn add_skip(&mut self, s: Skip) {
        self.skips.push(s);
    }
    fn do_skips(&mut self) {
        if self.skips.is_empty() {
            return;
        }
        let skips = std::mem::take(&mut self.skips);
        for skip in skips {
            match skip {
                Skip::String => skip_string(self),
                Skip::Array => skip_array(self),
                Skip::Object => skip_obj(self),
                Skip::ObjectValue { key_consumed } => skip_obj_value(self, key_consumed),
            }
        }
    }
}

fn next_any_item<'a>(b: u8, parse: &'a mut (dyn Parse + 'a)) -> JResult<'a> {
    match b {
        b'0'..=b'9' | b'-' => parse_number(parse, b),
        b'n' => parse_ident(parse, b"ull", Json::Null),
        b't' => parse_ident(parse, b"rue", Json::Bool(true)),
        b'f' => parse_ident(parse, b"alse", Json::Bool(false)),
        b'[' => Ok(Json::Array(ParseArray::new(parse))),
        b'{' => Ok(Json::Object(ParseObject::new(parse))),
        b'"' => Ok(Json::String(ParseString::new(parse))),
        b if b.is_ascii_alphabetic() => {
            parse.eat_until_whitespace();
            Err(SyntaxError::InvalidIdentifier.into())
        }
        other => panic!("unhandled {:?}", char::from(other)),
    }
}

fn parse_ident<'a>(parse: &mut dyn Parse, ident: &[u8], res: Json<'a>) -> JResult<'a> {
    for b in ident {
        let read = match parse.next_byte() {
            Some(b) => b,
            _ => return Err(SyntaxError::EofWhileParsingValue.into()),
        };
        if *b != read {
            parse.eat_until_whitespace();
            return Err(SyntaxError::InvalidIdentifier.into());
        }
    }
    Ok(res)
}

fn parse_number(parse: &mut dyn Parse, byte: u8) -> JResult {
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
        return Ok(Json::Number(Number::from(n)));
    }

    if let Ok(n) = s.parse::<i64>() {
        return Ok(Json::Number(Number::from(n)));
    }

    let n = s.parse::<f64>().unwrap();
    Ok(Json::Number(Number::from(n)))
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
    parse: Option<&'a mut dyn Parse>,
    ended: bool,
    needs_comma: bool,
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
        write!(
            f,
            "<{} for Parser@{:p}>",
            type_name::<Self>(),
            self.parse.as_ref().unwrap()
        )
    }
}
impl Debug for ParseObject<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{} for Parser@{:p}>",
            type_name::<Self>(),
            self.parse.as_ref().unwrap()
        )
    }
}

impl<'a> ParseArray<'a> {
    fn new(parse: &'a mut dyn Parse) -> Self {
        Self {
            parse: Some(parse),
            ended: false,
            needs_comma: false,
        }
    }

    pub fn next<'b>(&'b mut self) -> Option<JResult<'b>> {
        if self.ended {
            return None;
        }
        let parse: &'b mut (dyn Parse + 'a) = *self.parse.as_mut().unwrap();
        parse.do_skips();
        loop {
            let b = parse.peek_byte()?;
            match b {
                b']' => {
                    parse.next_byte();
                    self.ended = true;
                    return None;
                }
                b',' => {
                    parse.next_byte();
                    if self.needs_comma {
                        self.needs_comma = false;
                        continue;
                    } else {
                        return Some(Err(SyntaxError::TrailingComma.into()));
                    }
                }
                _ if b.is_ascii_whitespace() => {
                    parse.next_byte();
                    continue;
                }
                _ => {
                    if self.needs_comma {
                        self.needs_comma = false;
                        return Some(Err(SyntaxError::MissingComma.into()));
                    }
                    parse.next_byte();
                    self.needs_comma = true;
                    return Some(next_any_item(b, parse));
                }
            }
        }
    }
}

impl Drop for ParseArray<'_> {
    fn drop(&mut self) {
        if !self.ended {
            self.parse.as_mut().unwrap().add_skip(Skip::Array);
        }
    }
}

fn skip_array(parse: &mut dyn Parse) {
    let mut arr = ParseArray::new(parse);
    while arr.next().is_some() {}
}

pub struct ParseObject<'a> {
    parse: Option<&'a mut dyn Parse>,
    ended: bool,
}

impl<'a> ParseObject<'a> {
    fn new(parse: &'a mut dyn Parse) -> Self {
        Self {
            parse: Some(parse),
            ended: false,
        }
    }
    pub fn next(&mut self) -> Option<Result<KeyVal, Error>> {
        if self.ended {
            return None;
        }
        let parse: &mut dyn Parse = *self.parse.as_mut()?;
        parse.do_skips();
        loop {
            let b = parse.peek_byte()?;
            match b {
                _ if b.is_ascii_whitespace() || b == b',' => {
                    parse.next_byte();
                    continue;
                }
                b'}' => {
                    parse.next_byte();
                    self.ended = true;
                    return None;
                }
                b'"' => {
                    parse.next_byte();
                    break;
                }
                _ => panic!("unhandled char '{}' in object", char::from(b)),
            }
        }
        Some(Ok(KeyVal::new(parse)))
    }
}

impl<'a> Drop for ParseObject<'a> {
    fn drop(&mut self) {
        if !self.ended {
            self.parse.as_mut().unwrap().add_skip(Skip::Object);
        }
    }
}

fn skip_obj(parse: &mut dyn Parse) {
    let mut obj = ParseObject::new(parse);
    while obj.next().is_some() {}
}

/// Reads a key and/or value pair of an object.
///
/// They key and the value may be read independently, and either may be ignored.
///
/// For example, it's possible the only read the key, and ignore the value,
/// which will be skipped efficiently.
pub struct KeyVal<'a> {
    // None here means the object is exhausted
    parse: Option<&'a mut dyn Parse>,
    key_consumed: bool,
}

impl<'a> KeyVal<'a> {
    fn new(parse: &'a mut dyn Parse) -> Self {
        Self {
            parse: Some(parse),
            key_consumed: false,
        }
    }

    /// Begins parsing the current object key.
    /// Panics if called more than once.
    pub fn key(&mut self) -> ParseString {
        assert!(!self.key_consumed);
        self.key_consumed = true;
        ParseString::new(*self.parse.as_mut().unwrap())
    }

    /// Obtains a [`Json`] for this object value.
    /// Skips and discards the key if it was not already retrieved.
    pub fn value(mut self) -> JResult<'a> {
        let parse = self.parse.take().unwrap();
        read_value(parse, self.key_consumed)
    }
}

impl<'a> Drop for KeyVal<'a> {
    fn drop(&mut self) {
        if let Some(parse) = self.parse.as_mut() {
            parse.add_skip(Skip::ObjectValue {
                key_consumed: self.key_consumed,
            });
        }
    }
}

fn skip_obj_value(parse: &mut dyn Parse, key_consumed: bool) {
    let v = loop {
        if let Ok(v) = read_value(parse, key_consumed) {
            break v;
        }
    };
    match v {
        Json::String(mut p) => {
            p.skip();
        }
        Json::Array(mut p) => while p.next().is_some() {},
        Json::Object(mut p) => while p.next().is_some() {},
        _ => {}
    }
}

fn read_value(parse: &mut dyn Parse, key_consumed: bool) -> JResult {
    if !key_consumed {
        skip_string(parse);
    }

    parse.eat_whitespace();
    assert_eq!(parse.next_byte(), Some(b':'));
    parse.eat_whitespace();

    let b = match parse.next_byte() {
        Some(b) => b,
        _ => return Err(SyntaxError::EofWhileParsingValue.into()),
    };
    next_any_item(b, parse)
}

/// Reads a string. Reading can be done as a whole string,
/// or char-by-char if the string is expected to be very large.
pub struct ParseString<'a> {
    parse: Option<&'a mut dyn Parse>,
}

impl<'a> ParseString<'a> {
    fn new(parse: &'a mut dyn Parse) -> Self {
        Self { parse: Some(parse) }
    }

    /// Parses the entire JSON string into a new [`String`]
    pub fn read_owned(self) -> String {
        let mut buf = String::new();
        self.read_into(&mut buf).unwrap();
        buf
    }

    /// Parses the entire string into the supplied [`String`].
    /// This is useful to avoid allocating a new String,
    /// or to preallocate a buffer when the client code can guess the string length.
    pub fn read_into(mut self, buf: &mut String) -> Result<(), Error> {
        let parse = self.parse.take().unwrap();
        loop {
            match parse.next_byte() {
                None => break Err(SyntaxError::EofWhileParsingString.into()),
                Some(b'"') => break Ok(()),
                Some(c) => buf.push(c.into()),
            }
        }
    }

    /// Parses this JSON string one [`char`] at a time,
    /// instead of the entire string.
    pub fn read_chars(mut self) -> ParseChars<'a> {
        ParseChars::new(self.parse.take().unwrap())
    }

    fn skip(&mut self) {
        skip_string(*self.parse.as_mut().unwrap());
    }
}

impl Drop for ParseString<'_> {
    fn drop(&mut self) {
        if let Some(p) = self.parse.as_mut() {
            p.add_skip(Skip::String);
        }
    }
}

fn skip_string(parse: &mut dyn Parse) {
    let mut escape = false;
    while let Some(b) = parse.next_byte() {
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
    fn new(parse: &'a mut dyn Parse) -> Self {
        Self { parse }
    }

    fn unicode_escape(&mut self) -> Option<char> {
        let mut val = 0u32;
        loop {
            match self.parse.next_byte()? {
                c @ b'0'..=b'9' => val = val * 16 + u32::from(c - b'0'),
                c @ b'a'..=b'f' => val = val * 16 + u32::from(c - b'a') + 10,
                b'}' => return char::try_from(val).ok(),
                _ => return None,
            }
        }
    }
}

impl<'a> Iterator for ParseChars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let mut escape = false;
        loop {
            match self.parse.next_byte()? {
                b'\\' if !escape => {
                    escape = true;
                    continue;
                }
                b'"' if !escape => return None,
                b'r' if escape => return Some('\r'),
                b'u' if escape => match self.parse.next_byte()? {
                    b'{' => match self.unicode_escape() {
                        Some(c) => return Some(c),
                        _ => continue,
                    },
                    _ => continue,
                },
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

mod private {
    pub trait Sealed {}
}

pub trait JsonAccess<'a>: private::Sealed {
    #[inline]
    fn is_null(&self) -> bool {
        self.as_null().is_some()
    }
    #[inline]
    fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }
    #[inline]
    fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    fn is_string(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_object(&self) -> bool;

    fn as_null(&self) -> Option<()>;
    fn as_bool(&self) -> Option<bool>;
    fn as_number(&self) -> Option<Number>;

    fn as_string(self) -> Option<ParseString<'a>>;
    fn as_array(self) -> Option<ParseArray<'a>>;
    fn as_object(self) -> Option<ParseObject<'a>>;
}

impl private::Sealed for Json<'_> {}
impl<'a> JsonAccess<'a> for Json<'a> {
    fn as_null(&self) -> Option<()> {
        match self {
            Self::Null => Some(()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<Number> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    #[inline]
    fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    fn as_string(self) -> Option<ParseString<'a>> {
        match self {
            Self::String(a) => Some(a),
            _ => None,
        }
    }

    #[inline]
    fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    fn as_array(self) -> Option<ParseArray<'a>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    #[inline]
    fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    fn as_object(self) -> Option<ParseObject<'a>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }
}

impl private::Sealed for JResult<'_> {}
impl<'a> JsonAccess<'a> for JResult<'a> {
    fn as_null(&self) -> Option<()> {
        self.as_ref().ok()?.as_null()
    }

    fn as_bool(&self) -> Option<bool> {
        self.as_ref().ok()?.as_bool()
    }

    fn as_number(&self) -> Option<Number> {
        self.as_ref().ok()?.as_number()
    }

    #[inline]
    fn is_string(&self) -> bool {
        match self {
            Ok(j) => j.is_string(),
            _ => false,
        }
    }

    fn as_string(self) -> Option<ParseString<'a>> {
        self.ok().and_then(Json::as_string)
    }

    #[inline]
    fn is_array(&self) -> bool {
        match self {
            Ok(j) => j.is_array(),
            _ => false,
        }
    }

    fn as_array(self) -> Option<ParseArray<'a>> {
        self.ok().and_then(Json::as_array)
    }

    #[inline]
    fn is_object(&self) -> bool {
        match self {
            Ok(j) => j.is_object(),
            _ => false,
        }
    }

    fn as_object(self) -> Option<ParseObject<'a>> {
        self.ok().and_then(Json::as_object)
    }
}

#[derive(Debug)]
pub struct Error {
    err: Box<ErrorCode>,
}

impl Error {
    pub fn syntax(&self) -> Option<SyntaxError> {
        match *self.err {
            ErrorCode::Syntax(s) => Some(s),
            // _ => None,
        }
    }
}

impl From<SyntaxError> for Error {
    fn from(e: SyntaxError) -> Self {
        Self {
            err: Box::new(ErrorCode::Syntax(e)),
        }
    }
}

// Modeled after serde_json
#[derive(Debug)]
pub(crate) enum ErrorCode {
    /// Catchall for syntax error messages
    // Message(Box<str>),

    // Io(io::Error),
    Syntax(SyntaxError),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum SyntaxError {
    /// An unquoted string other than "null", "true", or "false" was encountered and skipped
    InvalidIdentifier,

    /// A character other than a collection close was encountered while looking for the next item
    MissingComma,

    /// EOF while parsing a list.
    EofWhileParsingList,

    /// EOF while parsing an object.
    EofWhileParsingObject,

    /// EOF while parsing a string.
    EofWhileParsingString,

    /// EOF while parsing a JSON value.
    EofWhileParsingValue,

    /// Expected this character to be a `':'`.
    ExpectedColon,

    /// Expected this character to be either a `','` or a `']'`.
    // ExpectedListCommaOrEnd,

    /// Expected this character to be either a `','` or a `'}'`.
    // ExpectedObjectCommaOrEnd,

    /// Expected to parse either a `true`, `false`, or a `null`.
    // ExpectedSomeIdent,

    /// Expected this character to start a JSON value.
    // ExpectedSomeValue,

    /// Invalid hex escape code.
    InvalidEscape,

    /// Invalid number.
    InvalidNumber,

    /// Number is bigger than the maximum value of its type.
    NumberOutOfRange,

    /// Invalid unicode code point.
    InvalidUnicodeCodePoint,

    /// Control character found while parsing a string.
    ControlCharacterWhileParsingString,

    /// Object key is not a string.
    KeyMustBeAString,

    /// Lone leading surrogate in hex escape.
    LoneLeadingSurrogateInHexEscape,

    /// JSON has a comma after the last value in an array or map.
    TrailingComma,

    /// JSON has non-whitespace trailing characters after the value.
    TrailingCharacters,

    /// Unexpected end of hex excape.
    UnexpectedEndOfHexEscape,

    /// Encountered nesting of JSON maps and arrays more than 128 layers deep.
    RecursionLimitExceeded,
}

macro_rules! impl_from_item {
    ( $(($ty:ty, $variant:ident)),* ) => {
        $(
            impl<'a> From<$ty> for Json<'a> {
                #[inline]
                fn from(x: $ty) -> Self {
                    Self::$variant(x)
                }
            }
        )*
    };
}

impl_from_item!(
    (bool, Bool),
    (Number, Number),
    (ParseString<'a>, String),
    (ParseArray<'a>, Array),
    (ParseObject<'a>, Object)
);
