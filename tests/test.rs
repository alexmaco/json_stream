use json_stream::parse::*;

#[test]
fn example() {
    let mut p = Parser::new(r#"["a","b","c"]"#.as_bytes());

    let mut arr = p
        .next()
        .unwrap()
        .as_array()
        .expect("expected root object to be an array");

    let mut seen: Vec<String> = vec![];

    while let Some(item) = arr.next() {
        if let Ok(Json::String(s)) = item {
            seen.push(s.read_owned());
        }
    }

    assert_eq!(seen, &["a", "b", "c"]);
}

#[test]
fn chars() {
    let mut p = Parser::new(r#""abc""#.as_bytes());

    let s = p
        .next()
        .unwrap()
        .as_string()
        .expect("expected root object to be a string");

    let chars: Vec<char> = s.read_chars().into_iter().collect();

    assert_eq!(chars, &['a', 'b', 'c']);
}

#[test]
#[ignore]
fn char_escapes() {
    let mut p = Parser::new(r#""\r\"\t""#.as_bytes());

    let s = p
        .next()
        .unwrap()
        .as_string()
        .expect("expected root object to be a string");

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

    assert!(p.next().is_none());
}

#[test]
fn empty_object_no_keyval() {
    let mut p = Parser::new("{ }".as_bytes());

    let mut obj = match p.next() {
        Some(Ok(Json::Object(obj))) => obj,
        _ => panic!("expected root object to be an object"),
    };

    assert!(obj.next().is_none());
}

#[test]
fn object_and_keyval() {
    let mut p = Parser::new(r#"{"a" : 2, "b":[3, 4], "c": false}"#.as_bytes());

    let mut obj = match p.next() {
        Some(Ok(Json::Object(obj))) => obj,
        _ => panic!("expected root object to be an object"),
    };

    let mut kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.key().read_owned(), "a");
    assert_eq!(kv.value().as_number(), Some(Number::from(2)));

    let kv = obj.next().unwrap().unwrap();
    assert!(kv.value().is_array());

    let mut kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.key().read_owned(), "c");
    drop(kv);

    assert!(obj.next().is_none());
}

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

#[test]
fn object_skipping() {
    let mut p = Parser::new(r#"{"a":{"x":2}, "b":3}"#.as_bytes());

    let mut obj = p
        .next()
        .unwrap()
        .as_object()
        .expect("expected root object to be an object");

    let mut kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.key().read_owned(), "a");
    drop(kv);

    let kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.value().as_number(), Some(Number::from(3)));
}

#[test]
fn missing_comma_error() {
    let mut p = Parser::new("[1 2]".as_bytes());

    let mut arr = p
        .next()
        .unwrap()
        .as_array()
        .expect("expected root object to be an array");

    assert_eq!(arr.next().unwrap().as_number(), Some(Number::from(1)));
    assert_eq!(
        arr.next().unwrap().unwrap_err().syntax(),
        Some(SyntaxError::MissingComma)
    );
    assert_eq!(arr.next().unwrap().as_number(), Some(Number::from(2)));
}

#[test]
fn trailing_comma_error() {
    let mut p = Parser::new("[1 , ,, 2]".as_bytes());

    let mut arr = p
        .next()
        .unwrap()
        .as_array()
        .expect("expected root object to be an array");

    assert_eq!(arr.next().unwrap().as_number(), Some(Number::from(1)));
    assert_eq!(
        arr.next().unwrap().unwrap_err().syntax(),
        Some(SyntaxError::TrailingComma)
    );
    assert_eq!(
        arr.next().unwrap().unwrap_err().syntax(),
        Some(SyntaxError::TrailingComma)
    );
    assert_eq!(arr.next().unwrap().as_number(), Some(Number::from(2)));
}

mod identifier_errors {
    use super::*;

    #[test]
    fn eof() {
        let mut p = Parser::new("nul".as_bytes());
        assert_eq!(
            p.next().unwrap().unwrap_err().syntax(),
            Some(SyntaxError::EofWhileParsingValue)
        );
    }

    #[test]
    fn invalid_ident() {
        let mut p = Parser::new("trxu false".as_bytes());
        assert_eq!(
            p.next().unwrap().unwrap_err().syntax(),
            Some(SyntaxError::InvalidIdentifier)
        );
        assert_eq!(p.next().unwrap().as_bool(), Some(false));
    }

    #[test]
    fn unknown_ident() {
        let mut p = Parser::new("potato false".as_bytes());
        assert_eq!(
            p.next().unwrap().unwrap_err().syntax(),
            Some(SyntaxError::InvalidIdentifier)
        );
        assert_eq!(p.next().unwrap().as_bool(), Some(false));
    }
}
