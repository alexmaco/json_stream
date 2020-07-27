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

use std::io::{ErrorKind, Read};

/// Reads bytes from a [`Read`], parses them as [`Json`], and returns a stream of values or sub-parsers via `fn next()`
pub struct Parser<R> {
    src: R,
}

impl<R: Read> Parser<R> {
    /// Constructs a new Parser that will read from the provided object.
    pub fn new(r: R) -> Self {
        Self { src: r }
    }

    /// Returns the next JSON item.
    /// A Parser will read any number of whitespace-separated JSON items and return them in order.
    /// Returns None when the input is exhausted.
    pub fn next(&mut self) -> Option<Json> {
        loop {
            let b = self.next_byte()?;
            break match next_any_item(b) {
                Some(f) => Some(f(self)),
                _ => continue,
            };
        }
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

fn next_any_item(b: u8) -> Option<fn(&mut dyn Parse) -> Json<'_>> {
    if b.is_ascii_whitespace() {
        return None;
    }

    Some(match b {
        b'0'..=b'9' | b'-' => |_| Json::Null,
        b'[' => |p| Json::Array(ParseArray { base: p }),
        b'{' => |p| Json::Object(ParseObject { base: p }),
        b'"' => |p| Json::String(ParseString { base: p }),
        other => panic!("unhandled {:?}", char::from(other)),
    })
}

pub struct Number(f64);
pub struct ParseArray<'a> {
    base: &'a mut dyn Parse,
}

impl<'a> ParseArray<'a> {
    pub fn next(&mut self) -> Option<Json> {
        loop {
            let b = self.base.next_byte()?;
            match b {
                b']' => return None,
                b',' => continue,
                _ => match next_any_item(b) {
                    Some(f) => return Some(f(self.base)),
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
pub enum Json<'a> {
    Null,
    Bool(bool),
    Number(Number),
    String(ParseString<'a>),
    Array(ParseArray<'a>),
    Object(ParseObject<'a>),
}
