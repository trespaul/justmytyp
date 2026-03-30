use super::*;

#[test]
fn test_request_deserialization() {
    let test = r#"{
            "name": "foo",
            "template": "bar",
            "input": { "bat": "baz" }
        }"#;

    let deserialized: RequestMessage = serde_json::from_str(test).expect("Malformed test data.");
    let raw_string: &str = deserialized.input.get();

    assert_eq!(raw_string, "{ \"bat\": \"baz\" }");
}

#[test]
#[allow(clippy::unreadable_literal)]
fn test_pdf_compile() {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let config = Config::init();
    let world = World::new(config.rootdir.clone(), config.cachedir.clone());

    let input = r#"{
            "title": "Hello, World!",
            "text": "I'm rendering Typst inside Rust!"
        }"#
    .to_owned();

    let template = String::from("./src/tests/test.typ");

    let compiled = pdf::compile(&world, template, input);

    let mut hasher = DefaultHasher::new();
    compiled.hash(&mut hasher);

    // TODO: this isn't deterministic probably?
    assert_eq!(
        hasher.finish(),
        8938572378811411970
    );
}
