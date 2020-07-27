use json_stream::parse::*;

#[test]
fn example() {
    let mut p = Parser::new(r#"["a","b","c"]"#.as_bytes());

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

#[test]
fn chars() {
    let mut p = Parser::new(r#""abc""#.as_bytes());

    let s = match p.next() {
        Some(Json::String(s)) => s,
        _ => panic!("expected root object to be an string"),
    };

    let chars: Vec<char> = s.read_chars().into_iter().collect();

    assert_eq!(chars, &['a', 'b', 'c']);
}
