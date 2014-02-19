use std::path::Path;

use tr_game::{MOD_LOAD_STATUS, ModLoad, load_original_mods};

#[test]
fn original_mod_load_is_unsupported() {
    assert_eq!(MOD_LOAD_STATUS, "unsupported");
    assert_eq!(load_original_mods(Path::new(".")), ModLoad::Unsupported);
    assert_eq!(ModLoad::Unsupported.as_str(), "unsupported");
}
