use tr_types::{ContentPath, ContentPathError};

#[test]
fn parse_and_display() {
    let p = ContentPath::parse("terraria::blocks::Dirt").unwrap();
    assert_eq!(p.to_string(), "terraria::blocks::Dirt");
    assert_eq!(p.name(), "Dirt");
    assert_eq!(p.namespace().to_string(), "terraria::blocks");
}

#[test]
fn display_is_stable_code() {
    let e = ContentPathError::Empty;
    assert_eq!(e.to_string(), "tr.types.content_path.empty");
}
