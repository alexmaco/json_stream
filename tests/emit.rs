#![allow(unused_must_use)]

use json_stream::emit::*;
use std::any::type_name;
use std::collections::{BTreeSet, BinaryHeap, HashMap, LinkedList, VecDeque};
use std::str::from_utf8;

#[test]
fn example() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let mut arr = e.array().unwrap();
        arr.emit("a");
        {
            let mut obj = arr.object().unwrap();
            obj.emit("k", "v");
        }
        arr.emit(&3);
    }

    assert_eq!(from_utf8(&buf).unwrap(), r#"["a",{"k":"v"},3]"#);
}

#[test]
fn commas_in_object() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let mut o = e.object().unwrap();
        o.emit("a", &1);
        o.emit("b", &2);
    }

    assert_eq!(from_utf8(&buf).unwrap(), r#"{"a":1,"b":2}"#);
}

#[test]
fn commas_near_arrays_in_object() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let mut o = e.object().unwrap();
        o.emit_array("a");
        let mut b = o.emit_array("b").unwrap();
        b.emit(&3);
        b.emit(&4);
    }

    assert_eq!(from_utf8(&buf).unwrap(), r#"{"a":[],"b":[3,4]}"#);
}

fn emit_thing_test<T: JsonEmit + ?Sized>(val: &T, expect: &str) {
    let mut buf = vec![];
    let mut e = Emitter::new(&mut buf);

    e.emit(val);

    assert_eq!(
        from_utf8(&buf).unwrap(),
        expect,
        "emitting failed for T={}",
        type_name::<T>()
    );
}

#[test]
fn basic_sequences() {
    let v: Vec<usize> = vec![1, 2, 3];
    emit_thing_test::<Vec<usize>>(&v, r#"[1,2,3]"#);

    let slice = &[1, 2, 3][..];
    emit_thing_test::<[usize]>(slice, r#"[1,2,3]"#);

    let arr = [1, 2, 3];
    emit_thing_test::<[usize; 3]>(&arr, r#"[1,2,3]"#);

    let deque = VecDeque::from([1, 2, 3]);
    emit_thing_test::<VecDeque<_>>(&deque, r#"[1,2,3]"#);

    let list = LinkedList::from([1, 2, 3]);
    emit_thing_test::<LinkedList<_>>(&list, r#"[1,2,3]"#);

    let tset = BTreeSet::from([1, 2, 3]);
    emit_thing_test::<BTreeSet<_>>(&tset, r#"[1,2,3]"#);

    let heap = BinaryHeap::from([1, 2, 3]);
    emit_thing_test::<BinaryHeap<_>>(&heap, r#"[3,2,1]"#);
}

#[test]
fn emitting_object() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let m = {
            let mut m = HashMap::new();
            m.insert("a", 1);
            m.insert("b", 2);
            m
        };
        e.emit(&m);
    }

    let result_str = from_utf8(&buf).unwrap();
    assert!(result_str == r#"{"a":1,"b":2}"# || result_str == r#"{"b":2,"a":1}"#);
}

#[test]
fn emitting_string() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);
        let s = String::from("abcd");
        e.emit(&s);
    }

    assert_eq!(from_utf8(&buf).unwrap(), r#""abcd""#);
}

#[test]
fn chars() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);
        let mut s = e.string().unwrap();
        s.char('a').unwrap();
        s.str("bcd").unwrap();
    }

    assert_eq!(from_utf8(&buf).unwrap(), r#""abcd""#);
}

#[test]
fn emitter_newline_between_items() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        e.emit(&3);
        e.emit("abc");
        e.array().unwrap().emit(&1);
        e.object().unwrap().emit("x", &5);
    }

    assert_eq!(
        from_utf8(&buf).unwrap(),
        r#"3
"abc"
[1]
{"x":5}"#
    );
}

#[test]
fn commas_in_array() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);
        let mut arr = e.array().unwrap();
        arr.string().unwrap().str("abc");
        arr.string().unwrap().str("def");
    }

    assert_eq!(from_utf8(&buf).unwrap(), r#"["abc","def"]"#);
}

#[test]
fn emitter_newline_after_string() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        e.string().unwrap().str("abc");
        e.string().unwrap().str("def");
    }

    assert_eq!(
        from_utf8(&buf).unwrap(),
        r#""abc"
"def""#
    );
}
