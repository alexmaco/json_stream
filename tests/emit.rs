use json_stream::emit::*;
use std::collections::HashMap;

#[test]
fn example() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let mut arr = e.array();
        arr.emit("a");
        {
            let mut obj = arr.object();
            obj.emit("k", "v");
        }
        arr.emit(&3);
    }

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#"["a",{"k":"v"},3]"#);
}

#[test]
fn commas_in_object() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let mut o = e.object();
        o.emit("a", &1);
        o.emit("b", &2);
    }

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#"{"a":1,"b":2}"#);
}

#[test]
fn commas_near_arrays_in_object() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let mut o = e.object();
        o.emit_array("a");
        let mut b = o.emit_array("b");
        b.emit(&3);
        b.emit(&4);
    }

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#"{"a":[],"b":[3,4]}"#);
}

#[test]
fn emitting_vecs() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        let v: Vec<usize> = vec![1, 2, 3];
        e.emit(&v);
    }

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#"[1,2,3]"#);
}

#[test]
fn emitting_slice() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        e.emit(&[1, 2, 3][..]);
    }

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#"[1,2,3]"#);
}

#[test]
fn emitting_array() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        e.emit(&[1, 2, 3]);
    }

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#"[1,2,3]"#);
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

    let result_str = std::str::from_utf8(&buf).unwrap();
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

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#""abcd""#);
}

#[test]
fn emitter_newline_between_items() {
    let mut buf = vec![];
    {
        let mut e = Emitter::new(&mut buf);

        e.emit(&3);
        e.emit("abc");
        e.array().emit(&1);
        e.object().emit("x", &5);
    }

    assert_eq!(
        std::str::from_utf8(&buf).unwrap(),
        r#"3
"abc"
[1]
{"x":5}"#
    );
}
