use json_stream::parse::*;

fn main() {
    let mut p = Parser::new(r#"{"a":2, "b":3}"#.as_bytes());

    let mut obj = p
        .next()
        .unwrap()
        .as_object()
        .expect("expected root object to be an array");

    let _kv = obj.next().unwrap();

    obj.next();
}
