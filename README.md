# JSON Stream

A streaming JSON parser/emitter for rust.

## Why ?

* to process a 50GiB json on a machine with 16GiB of ram
* to process a 10MiB json on an embedded device with 2MiB of ram
* to read and write large amounts of json off a socket directly, without the need to buffer

## Features

* Streaming parser
* Streaming emitter [todo]
* sync and async support [todo]
* `#[no_std]` support [todo]
* optionally tolerates and recovers from errors [todo]
* optional `serde` integration via the `serde` feature [todo]

## Examples

```rust
let s = r#"["a", "b", "c"]"#;
let mut p = Parser::new(s.as_bytes());

let mut arr = p
    .next()
    .unwrap()
    .as_array()
    .expect("expected root object to be an array");

let mut seen: Vec<String> = vec![];

while let Some(item) = arr.next() {
    if let Json::String(s) = item {
        seen.push(s.read_owned());
    }
}

assert_eq!(seen, &["a", "b", "c"]);
```

## In development

Still under development, API changes can happen before 1.0, and optimizations still need to be done
