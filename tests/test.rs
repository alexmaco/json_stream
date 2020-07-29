use json_stream::parse::*;

#[test]
fn example() {
    let mut p = Parser::new(r#"["a","b","c"]"#.as_bytes());

    let mut json = p.next().unwrap();
    let mut arr = json
        .as_array()
        .expect("expected root object to be an array");

    let mut seen: Vec<String> = vec![];

    while let Some(item) = arr.next() {
        if let Json::String(s) = item {
            seen.push(s.read_owned());
        }
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
#[ignore]
fn char_escapes() {
    let mut p = Parser::new(r#""\r\"\t""#.as_bytes());

    let s = match p.next() {
        Some(Json::String(s)) => s,
        _ => panic!("expected root object to be an string"),
    };

    let chars: Vec<char> = s.read_chars().into_iter().collect();

    assert_eq!(chars, &['\\', 'r', '"', '\\', 't']);
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

#[test]
fn empty_object_no_keyval() {
    let mut p = Parser::new("{ }".as_bytes());

    let mut obj = match p.next() {
        Some(Json::Object(obj)) => obj,
        _ => panic!("expected root object to be an array"),
    };

    assert!(obj.next().is_none());
}

#[test]
fn object_and_keyval() {
    let mut p = Parser::new(r#"{"a" : 2, "b":[3, 4], "c": false}"#.as_bytes());

    let mut obj = match p.next() {
        Some(Json::Object(obj)) => obj,
        _ => panic!("expected root object to be an array"),
    };

    let mut kv = obj.next().unwrap();
    assert_eq!(kv.key().read_owned(), "a");
    assert_eq!(kv.value().as_number(), Some(Number::from(2)));

    let kv = obj.next().unwrap();
    assert!(kv.value().is_array());

    let mut kv = obj.next().unwrap();
    assert_eq!(kv.key().read_owned(), "c");
    drop(kv);

    assert!(obj.next().is_none());
}
