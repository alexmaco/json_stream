use std::io::Write;

pub struct Emitter<W: Write> {
    dst: W,
}

impl<W: Write> Emitter<W> {
    /// Constructs a new Emitter that will write to the provided Write.
    pub fn new(dst: W) -> Self {
        Self { dst }
    }
}

impl<W: Write> Emit for Emitter<W> {
    fn array(&mut self) -> EmitArray {
        EmitArray::new(self)
    }
    fn object(&mut self) -> EmitObject {
        EmitObject::new(self)
    }
}

impl<'a> Emit for EmitArray<'a> {
    fn array(&mut self) -> EmitArray {
        self.start();
        EmitArray::new(self.emit)
    }
    fn object(&mut self) -> EmitObject {
        self.start();
        EmitObject::new(self.emit)
    }
}

pub trait Emit {
    fn array(&mut self) -> EmitArray;
    fn object(&mut self) -> EmitObject;
}

#[doc(hidden)]
pub trait EmitData {
    fn put(&mut self, b: u8);
}

impl<W: Write> EmitData for Emitter<W> {
    fn put(&mut self, b: u8) {
        self.dst.write(&[b]).unwrap();
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

    pub fn emit<T: JsonEmit>(&mut self, value: T) {
        self.start();
        value.write_to(self.emit)
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
    fn key<S>(&mut self, key: S)
    where
        S: AsRef<str>,
    {
        self.start();
        key.as_ref().write_to(self.emit);
        self.emit.put(b':');
    }

    pub fn emit<S, V>(&mut self, key: S, value: V)
    where
        S: AsRef<str>,
        V: JsonEmit,
    {
        self.key(key);
        value.write_to(self.emit);
    }

    pub fn emit_array<S>(&mut self, key: S) -> EmitArray
    where
        S: AsRef<str>,
    {
        self.key(key);
        EmitArray::new(self.emit)
    }

    pub fn emit_object<S>(&mut self, key: S) -> EmitObject
    where
        S: AsRef<str>,
    {
        self.key(key);
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

pub trait JsonEmit: private::Sealed {
    #[doc(hidden)]
    fn write_to(self, emit: &mut dyn EmitData);
}

impl private::Sealed for &str {}
impl JsonEmit for &str {
    fn write_to(self, emit: &mut dyn EmitData) {
        emit.put(b'"');
        for b in self.as_bytes() {
            emit.put(*b)
        }
        emit.put(b'"');
    }
}

impl private::Sealed for usize {}
impl JsonEmit for usize {
    fn write_to(self, emit: &mut dyn EmitData) {
        let tmp = format!("{}", self);
        for b in tmp.as_bytes() {
            emit.put(*b)
        }
    }
}

// enum ValWrap<'a> {
//     Null,
//     Bool(bool),
//     Usize(usize),
//     Str(&'a str),
// }

// impl<'a> From<&'a str> for ValWrap<'a> {
//     fn from(s: &'a str) -> Self {
//         Self::Str(s)
//     }
// }
