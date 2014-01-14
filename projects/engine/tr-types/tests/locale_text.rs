use tr_types::LocaleText;

#[test]
fn splits_standard_block_path() {
    let t = LocaleText::new("standard.block.dirt");
    let r = t.message_ref();
    assert_eq!(r.namespace.as_str(), "standard");
    assert_eq!(r.to_string(), "standard.block.dirt");
}
