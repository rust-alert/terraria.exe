#[test]
fn exports() {
    assert!((tr_wasm::tr_vec2_length(3.0, 4.0) - 5.0).abs() < 1e-9);
    assert_eq!(tr_wasm::tr_version_code(), 0);
}
