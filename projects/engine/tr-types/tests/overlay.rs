use tr_types::{BlockFieldOverride, BlockTexture, LocaleText, TerrariaBlock, merge_block};

#[test]
fn dirt_only_overrides_name() {
    let base = TerrariaBlock::default();
    let merged = merge_block(
        &base,
        &BlockFieldOverride {
            name: Some(LocaleText::new("standard.block.dirt")),
            ..Default::default()
        },
    );
    assert_eq!(merged.name.symbol.as_str(), "standard.block.dirt");
    assert!(merged.solid);
    assert_eq!(merged.max_hp, 50);
    assert_eq!(merged.light_radius, 0);
}

#[test]
fn torch_overrides_light_and_solidity() {
    let merged = merge_block(
        &TerrariaBlock::default(),
        &BlockFieldOverride {
            name: Some(LocaleText::new("standard.block.torch")),
            solid: Some(false),
            blocks_motion: Some(false),
            light_radius: Some(8),
            max_hp: Some(10),
            ..Default::default()
        },
    );
    assert!(!merged.solid);
    assert_eq!(merged.light_radius, 8);
}

#[test]
fn grass_overrides_faces() {
    let merged = merge_block(
        &TerrariaBlock::default(),
        &BlockFieldOverride {
            name: Some(LocaleText::new("standard.block.grass")),
            texture: Some(BlockTexture::faces(
                "asset.textures.dirt",
                "asset.textures.grass_top",
                "asset.textures.grass_side",
                "asset.textures.dirt",
            )),
            ..Default::default()
        },
    );
    assert_eq!(
        merged.texture.top.as_deref(),
        Some("asset.textures.grass_top")
    );
    assert_eq!(merged.texture.default.as_ref(), "asset.textures.dirt");
}
