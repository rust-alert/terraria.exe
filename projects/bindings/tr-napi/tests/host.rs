use tr_napi::TerrariaJsHost;

#[test]
fn info_and_geometry() {
    let host = TerrariaJsHost::new();
    assert_eq!(host.info().npm_package, "@game-gpt/terraria");
    assert!((host.vec2_length(3.0, 4.0) - 5.0).abs() < 1e-5);
}

#[test]
fn reject_missing_original() {
    let host = TerrariaJsHost::new();
    let err = host
        .validate_path("Z:/definitely-not-terraria")
        .unwrap_err();
    assert!(err.contains("目录") || err.contains("Content") || err.contains("安装"));
}
