use std::path::PathBuf;

use super::*;

#[test]
fn test_request_deserialization() {
    let test = r#"{
            "name": "foo",
            "template": "bar",
            "input": { "bat": "baz" }
        }"#;

    let deserialized: RequestMessage = serde_json::from_str(test).unwrap();
    let raw_string: &str = deserialized.input.get();

    assert_eq!(raw_string, "{ \"bat\": \"baz\" }");
}

#[test]
fn test_pdf_compile() {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let input = r#"{
            "title": "Hello, World!",
            "text": "I'm rendering Typst inside Rust!"
        }"#
    .to_owned();

    let template = String::from("./src/tests/test.typ");

    let config = Config {
        loglevel: log::LevelFilter::Debug,
        rootdir: PathBuf::from("./"),
        cachedir: PathBuf::from("./.cache"),
        timeout: 10,
        bindaddress: String::from("0.0.0.0:3000"),
        timestampformat: "".to_string(),
        s3: None,
    };

    let compiled = pdf::compile(template, input, &config);

    let mut hasher = DefaultHasher::new();
    compiled.hash(&mut hasher);

    // TODO: this isn't deterministic probably
    assert_eq!(hasher.finish(), 8938572378811411970);
}
