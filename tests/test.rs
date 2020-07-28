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

#[test]
fn basics() {
    let mut p = Parser::new("null true false 0 1 -2 6.28".as_bytes());

    assert!(p.next().unwrap().is_null());
    assert_eq!(p.next().unwrap().as_bool(), Some(true));
    assert_eq!(p.next().unwrap().as_bool(), Some(false));
    assert_eq!(p.next().unwrap().as_number(), Some(Number::from(0)));
    assert_eq!(p.next().unwrap().as_number(), Some(Number::from(1)));
    assert_eq!(p.next().unwrap().as_number(), Some(Number::from(-2)));
    assert_eq!(p.next().unwrap().as_number(), Some(Number::from(6.28)));
}
