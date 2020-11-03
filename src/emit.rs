use std::io::Write;

pub struct Emitter<W: Write> {
    dst: W,
}

impl<W: Write> Emitter<W> {
    /// Constructs a new Emitter that will write to the provided Write.
    pub fn new(dst: W) -> Self {
        Self { dst }
    }

    pub fn array(&mut self) -> EmitArray {
        EmitArray::new(self)
    }
}

#[doc(hidden)]
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

    pub fn emit<S, V>(&mut self, key: S, value: V)
    where
        S: AsRef<str>,
        V: JsonEmit,
    {
        key.as_ref().write_to(self.emit);
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
    pub trait Sealed {}
}

pub trait JsonEmit: private::Sealed {
    #[doc(hidden)]
    fn write_to(self, emit: &mut dyn Emit);
}

impl private::Sealed for &str {}
impl JsonEmit for &str {
    fn write_to(self, emit: &mut dyn Emit) {
        emit.put(b'"');
        for b in self.as_bytes() {
            emit.put(*b)
        }
        emit.put(b'"');
    }
}

impl private::Sealed for usize {}
impl JsonEmit for usize {
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
