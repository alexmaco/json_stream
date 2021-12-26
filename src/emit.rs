use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::io::{self, Write};

pub struct Emitter<W: Write> {
    dst: W,
    started: bool,
}

impl<W: Write> Emitter<W> {
    /// Constructs a new Emitter that will write to the provided Write.
    pub fn new(dst: W) -> Self {
        Self {
            dst,
            started: false,
        }
    }

    fn start(&mut self) {
        if !self.started {
            self.started = true;
        } else {
            self.put(b'\n');
        }
    }
}

impl<W: Write> Emit for Emitter<W> {
    fn emit<T: JsonEmit + ?Sized>(&mut self, value: &T) {
        self.start();
        value.write_to(self)
    }

    fn string(&mut self) -> EmitString {
        // self.start()
        EmitString::new(self)
    }

    fn array(&mut self) -> EmitArray {
        self.start();
        EmitArray::new(self)
    }

    fn object(&mut self) -> EmitObject {
        self.start();
        EmitObject::new(self)
    }
}

impl<'a> Emit for EmitArray<'a> {
    fn emit<T: JsonEmit + ?Sized>(&mut self, value: &T) {
        self.start();
        value.write_to(self.emit)
    }

    fn string(&mut self) -> EmitString {
        // self.start()
        EmitString::new(self.emit)
    }

    fn array(&mut self) -> EmitArray {
        self.start();
        EmitArray::new(self.emit)
    }

    fn object(&mut self) -> EmitObject {
        self.start();
        EmitObject::new(self.emit)
    }
}

/// Provides methods that can be used to emit a value inside the current value.
/// [EmitObject] does not use this trait because it needs to emit key-value pairs.
pub trait Emit {
    fn emit<T: JsonEmit + ?Sized>(&mut self, value: &T);

    fn string(&mut self) -> EmitString;

    fn array(&mut self) -> EmitArray;

    fn object(&mut self) -> EmitObject;
}

#[doc(hidden)]
pub trait EmitData {
    fn put(&mut self, b: u8);
    fn write(&mut self) -> &mut dyn Write;
}

impl<W: Write> EmitData for Emitter<W> {
    fn put(&mut self, b: u8) {
        self.dst.write_all(&[b]).unwrap();
    }
    fn write(&mut self) -> &mut dyn Write {
        self.dst.by_ref()
    }
}

pub struct EmitString<'a> {
    emit: &'a mut dyn EmitData,
}

impl<'a> EmitString<'a> {
    fn new(emit: &'a mut dyn EmitData) -> Self {
        emit.put(b'"');
        Self { emit }
    }

    pub fn char(&mut self, c: char) -> Result {
        write!(self.emit.write(), "{}", c).map_err(Error::from)
    }

    pub fn str(&mut self, s: &str) -> Result {
        write!(self.emit.write(), "{}", s).map_err(Error::from)
    }
}

impl Drop for EmitString<'_> {
    fn drop(&mut self) {
        self.emit.put(b'"')
    }
}

pub struct EmitArray<'a> {
    emit: &'a mut dyn EmitData,
    started: bool,
}

impl<'a> EmitArray<'a> {
    fn new(emit: &'a mut dyn EmitData) -> Self {
        emit.put(b'[');
        Self {
            emit,
            started: false,
        }
    }

    fn start(&mut self) {
        if !self.started {
            self.started = true;
        } else {
            self.emit.put(b',');
        }
    }
}

impl Drop for EmitArray<'_> {
    fn drop(&mut self) {
        self.emit.put(b']')
    }
}

pub struct EmitObject<'a> {
    emit: &'a mut dyn EmitData,
    started: bool,
}

impl<'a> EmitObject<'a> {
    fn new(emit: &'a mut dyn EmitData) -> Self {
        emit.put(b'{');
        Self {
            emit,
            started: false,
        }
    }

    fn start(&mut self) {
        if !self.started {
            self.started = true;
        } else {
            self.emit.put(b',');
        }
    }

    #[inline(always)]
    fn emit_key<S>(&mut self, key: S)
    where
        S: AsRef<str>,
    {
        self.start();
        key.as_ref().write_to(self.emit);
        self.emit.put(b':');
    }

    pub fn emit<S, V>(&mut self, key: S, value: &V)
    where
        S: AsRef<str>,
        V: JsonEmit + ?Sized,
    {
        self.emit_key(key);
        value.write_to(self.emit);
    }

    pub fn emit_array<S>(&mut self, key: S) -> EmitArray
    where
        S: AsRef<str>,
    {
        self.emit_key(key);
        EmitArray::new(self.emit)
    }

    pub fn emit_object<S>(&mut self, key: S) -> EmitObject
    where
        S: AsRef<str>,
    {
        self.emit_key(key);
        EmitObject::new(self.emit)
    }
}

impl Drop for EmitObject<'_> {
    fn drop(&mut self) {
        self.emit.put(b'}')
    }
}

mod private {
    pub trait Sealed {}
}

/// Implemented for primitve and standard library types that can be emitted as JSON
pub trait JsonEmit: private::Sealed {
    #[doc(hidden)]
    fn write_to(&self, emit: &mut dyn EmitData);
}

macro_rules! impl_json_emit_via_string_format {
    ( $($ty:ty),* ) => {
        $(
            impl private::Sealed for $ty {}
            impl JsonEmit for $ty {
                fn write_to(&self, emit: &mut dyn EmitData) {
                    write!(emit.write(), "{}", self).unwrap();
                }
            }
        )*
    };
}

impl_json_emit_via_string_format!(
    usize, isize, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64, char, bool
);

impl private::Sealed for str {}
impl JsonEmit for str {
    fn write_to(&self, emit: &mut dyn EmitData) {
        emit.put(b'"');
        for b in self.as_bytes() {
            emit.put(*b)
        }
        emit.put(b'"');
    }
}

impl private::Sealed for String {}
impl JsonEmit for String {
    fn write_to(&self, emit: &mut dyn EmitData) {
        emit.put(b'"');
        for b in self.as_bytes() {
            emit.put(*b)
        }
        emit.put(b'"');
    }
}

macro_rules! impl_json_emit_for_generic_seq {
    ( $ty:ty ) => {
        impl<T> private::Sealed for $ty where T: JsonEmit {}
        impl<T> JsonEmit for $ty
        where
            T: JsonEmit,
        {
            fn write_to(&self, emit: &mut dyn EmitData) {
                let mut a = EmitArray::new(emit);
                for val in self {
                    a.emit(val)
                }
            }
        }
    };
}

impl_json_emit_for_generic_seq!([T]);
impl_json_emit_for_generic_seq!(Vec<T>);
impl_json_emit_for_generic_seq!(VecDeque<T>);
impl_json_emit_for_generic_seq!(LinkedList<T>);
impl_json_emit_for_generic_seq!(HashSet<T>);
impl_json_emit_for_generic_seq!(BTreeSet<T>);
impl_json_emit_for_generic_seq!(BinaryHeap<T>);

impl<T, const N: usize> private::Sealed for [T; N] where T: JsonEmit {}
impl<T, const N: usize> JsonEmit for [T; N]
where
    T: JsonEmit,
{
    #[inline(always)]
    fn write_to(&self, emit: &mut dyn EmitData) {
        self.as_slice().write_to(emit);
    }
}

macro_rules! impl_json_emit_for_generic_map {
    ( $ty:ty ) => {
        impl<K, V> private::Sealed for $ty {}
        impl<K, V> JsonEmit for $ty
        where
            K: AsRef<str>,
            V: JsonEmit,
        {
            fn write_to(&self, emit: &mut dyn EmitData) {
                let mut o = EmitObject::new(emit);
                for (k, v) in self {
                    o.emit(k, v);
                }
            }
        }
    };
}

impl_json_emit_for_generic_map!(HashMap<K, V>);
impl_json_emit_for_generic_map!(BTreeMap<K, V>);

type Result = std::result::Result<(), Error>;

#[derive(Debug)]
pub struct Error(Box<ErrorCode>);

// Modeled after serde_json
#[non_exhaustive]
#[derive(Debug)]
pub(crate) enum ErrorCode {
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self(Box::new(ErrorCode::Io(e)))
    }
}
