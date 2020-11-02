// use std::borrow::Cow;
use std::io::Write;

/// Writes bytes to a [`Read`], parses them as [`Json`], and returns a stream of values or sub-parsers via `fn next()`
pub struct Emitter<W: Write> {
    dst: W,
}

impl<W: Write> Emitter<W> {
    /// Constructs a new Emitter that will write from the provided Write.
    pub fn new(dst: W) -> Self {
        Self { dst }
    }

    pub fn array(&mut self) -> EmitArray {
        EmitArray::new(self)
    }
}

pub trait Emit {
    fn put(&mut self, b: u8);
}

impl<W: Write> Emit for Emitter<W> {
    fn put(&mut self, b: u8) {
        self.dst.write(&[b]).unwrap();
    }
}

pub struct EmitArray<'a> {
    emit: &'a mut dyn Emit,
    started: bool,
}

impl<'a> EmitArray<'a> {
    fn new(emit: &'a mut dyn Emit) -> Self {
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

    pub fn emit_obj(&mut self) -> EmitObject {
        self.start();
        EmitObject::new(self.emit)
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
    emit: &'a mut dyn Emit,
}

impl<'a> EmitObject<'a> {
    fn new(emit: &'a mut dyn Emit) -> Self {
        emit.put(b'{');
        Self { emit }
    }

    // pub fn emit<S: for<'r> Into<Cow<'r, str>>, T: JsonEmit>(&mut self, key: S, value: T) {
    //     let cow = key.into();
    //     cow.write
    //     value.write_to(self.emit)
    // }

    pub fn emit<T: JsonEmit>(&mut self, key: &str, value: T) {
        private::Sealed::write_to(key, self.emit);
        self.emit.put(b':');
        value.write_to(self.emit);
    }
}

impl Drop for EmitObject<'_> {
    fn drop(&mut self) {
        self.emit.put(b'}')
    }
}

mod private {
    use super::Emit;
    pub trait Sealed {
        fn write_to(self, emit: &mut dyn Emit);
    }
}

pub trait JsonEmit: private::Sealed {}

impl JsonEmit for &str {}
impl private::Sealed for &str {
    fn write_to(self, emit: &mut dyn Emit) {
        emit.put(b'"');
        for b in self.as_bytes() {
            emit.put(*b)
        }
        emit.put(b'"');
    }
}
impl JsonEmit for usize {}
impl private::Sealed for usize {
    fn write_to(self, emit: &mut dyn Emit) {
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
