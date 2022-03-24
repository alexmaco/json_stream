use json_stream::parse::*;

#[test]
fn example() {
    let mut p = Parser::new(r#"["a","b","c"]"#.as_bytes());

    let mut arr = p
        .next()
        .as_array()
        .expect("expected root value to be an array");

    let mut seen: Vec<String> = vec![];

    while let Some(item) = arr.next() {
        if let Ok(Json::String(s)) = item {
            seen.push(s.read_owned().expect("cannot read string"));
        }
    }

    assert_eq!(seen, &["a", "b", "c"]);
}

#[test]
fn chars() {
    let mut p = Parser::new(r#""abc""#.as_bytes());

    let s = p
        .next()
        .as_string()
        .expect("expected root value to be a string");

    let chars: Vec<char> = s.read_chars().collect();

    assert_eq!(chars, &['a', 'b', 'c']);
}

#[test]
fn chars_into_string() {
    let mut p = Parser::new(r#""abc""#.as_bytes());

    let s = p
        .next()
        .as_string()
        .expect("expected root value to be a string");

    let chars: String = s.read_chars().collect();

    assert_eq!(chars, "abc");
}

#[test]
fn string_with_escapes() {
    let mut p = Parser::new(r#""a\"bc""#.as_bytes());

    let s = p
        .next()
        .as_string()
        .expect("expected root value to be a string")
        .read_owned()
        .expect("cannot read");

    assert_eq!(s, "a\"bc");
}

fn test_single_char(expected: char, s: &str) {
    let parsed = format!(r#""{}""#, s);
    let mut p = Parser::new(parsed.as_bytes());

    let str_parser = p
        .next()
        .as_string()
        .expect("expected root value to be a string");

    let chars: Vec<char> = str_parser.read_chars().into_iter().collect();

    dbg!(&chars, s, expected, expected.escape_unicode().to_string());
    assert_eq!(chars, &[expected]);
}

#[test]
fn char_escapes() {
    let pairs = [
        ('\\', r#"\\"#),
        ('"', r#"\""#),
        ('\r', r#"\r"#),
        ('\u{1234}', r#"\u{1234}"#),
        ('\u{ab34}', r#"\u{ab34}"#),
        ('\u{AB34}', r#"\u{ab34}"#),
    ];

    for (c, s) in &pairs {
        test_single_char(*c, s);
    }
}

#[test]
fn basics() {
    let mut p = Parser::new("null true false 0 1 -2 6.28".as_bytes());

    assert!(p.next().is_null());
    assert_eq!(p.next().as_bool(), Some(true));
    assert_eq!(p.next().as_bool(), Some(false));
    assert_eq!(p.next().as_number(), Some(Number::from(0)));
    assert_eq!(p.next().as_number(), Some(Number::from(1)));
    assert_eq!(p.next().as_number(), Some(Number::from(-2)));
    assert_eq!(p.next().as_number(), Some(Number::from(6.28)));

    assert!(p.next().is_none());
}

#[test]
fn empty_object_no_keyval() {
    let mut p = Parser::new("{ }".as_bytes());

    let mut obj = match p.next() {
        Some(Ok(Json::Object(obj))) => obj,
        _ => panic!("expected root value to be an object"),
    };

    assert!(obj.next().is_none());
}

#[test]
fn object_and_keyval() {
    let mut p = Parser::new(r#"{"a" : 2, "b":[3, 4], "c": false}"#.as_bytes());

    let mut obj = match p.next() {
        Some(Ok(Json::Object(obj))) => obj,
        _ => panic!("expected root value to be an object"),
    };

    let mut kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.key().read_owned(), Ok("a".to_owned()));
    assert_eq!(kv.value().as_number(), Some(Number::from(2)));

    let kv = obj.next().unwrap().unwrap();
    assert!(kv.value().is_array());

    let mut kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.key().read_owned(), Ok("c".to_owned()));
    drop(kv);

    assert!(obj.next().is_none());
}

#[test]
fn object_skipping() {
    let mut p = Parser::new(r#"{"a":{"x":2}, "b":3}"#.as_bytes());

    let mut obj = p
        .next()
        .as_object()
        .expect("expected root value to be an object");

    let mut kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.key().read_owned(), Ok("a".to_owned()));
    drop(kv);

    let kv = obj.next().unwrap().unwrap();
    assert_eq!(kv.value().as_number(), Some(Number::from(3)));
}

#[test]
fn array_skipping() {
    let mut p = Parser::new(r#"[1, [2,3], 4]"#.as_bytes());

    let mut arr = p
        .next()
        .as_array()
        .expect("expected root value to be an array");

    let one = arr.next().as_number();
    assert_eq!(one, Some(Number::from(1u32)));

    let mut sub_arr = arr.next().as_array().unwrap();
    let two = sub_arr.next().as_number();
    assert_eq!(two, Some(Number::from(2u32)));
    drop(sub_arr);

    let four = arr.next().as_number();
    assert_eq!(four, Some(Number::from(4u32)));
}

#[test]
fn missing_comma_error() {
    let mut p = Parser::new("[1 2]".as_bytes());

    let mut arr = p
        .next()
        .as_array()
        .expect("expected root value to be an array");

    assert_eq!(arr.next().as_number(), Some(Number::from(1)));
    assert_eq!(
        arr.next().unwrap().unwrap_err().syntax(),
        Some(SyntaxError::MissingComma)
    );
    assert_eq!(arr.next().as_number(), Some(Number::from(2)));
}

#[test]
fn trailing_comma_error() {
    let mut p = Parser::new("[1 , ,, 2]".as_bytes());

    let mut arr = p
        .next()
        .unwrap()
        .as_array()
        .expect("expected root value to be an array");

    assert_eq!(arr.next().as_number(), Some(Number::from(1)));
    assert_eq!(
        arr.next().unwrap().unwrap_err().syntax(),
        Some(SyntaxError::TrailingComma)
    );
    assert_eq!(
        arr.next().unwrap().unwrap_err().syntax(),
        Some(SyntaxError::TrailingComma)
    );
    assert_eq!(arr.next().as_number(), Some(Number::from(2)));
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
