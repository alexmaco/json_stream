use json_stream::parse::*;

fn main() {
    let mut p = Parser::new(r#"["a","b","c"]"#.as_bytes());

    let json = p.next().unwrap();
    let _arr = json
        .as_array()
        .expect("expected root value to be an array");

    let _j2 = p.next();
}
