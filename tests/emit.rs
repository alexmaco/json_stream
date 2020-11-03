use json_stream::emit::*;

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
        arr.emit(3);
    }

    assert_eq!(std::str::from_utf8(&buf).unwrap(), r#"["a",{"k":"v"},3]"#);
}
