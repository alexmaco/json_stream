use json_stream::parse::*;

#[test]
fn example() {
    let s = String::from(r#"["a","b","c"]"#);
    let mut p = Parser::new(s.as_bytes());

    let mut arr = match p.next() {
        Some(Json::Array(seq)) => seq,
        _ => panic!("expected root object to be an array"),
    };

    let mut seen: Vec<String> = vec![];

    while let Some(item) = arr.next() {
        let s = match item {
            Json::String(s) => s,
            _ => continue,
        };
        seen.push(s.read_owned());
    }

    assert_eq!(seen, &["a", "b", "c"]);
}
