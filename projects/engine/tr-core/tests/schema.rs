use tr_core::{block_def_from_schema, example_dirt_block};
use tr_types::{
    BlockFieldOverride, BlockTexture, ContentPath, LocaleText, TerrariaBlock, merge_block,
};

#[test]
fn dirt_schema_materializes_registry_def() {
    let (path, block) = example_dirt_block();
    let def = block_def_from_schema(&path, &block);
    assert_eq!(def.key, "terraria::blocks::Dirt");
    assert_eq!(def.name, "standard.block.dirt");
    assert!(def.solid);
    assert_eq!(def.max_hp, 50);
}

#[test]
fn grass_faces_materialize() {
    let path = ContentPath::parse("terraria::blocks::Grass").unwrap();
    let block = merge_block(
        &TerrariaBlock::default(),
        &BlockFieldOverride {
            name: Some(LocaleText::new("standard.block.grass")),
            texture: Some(BlockTexture::faces(
                "textures/dirt.png",
                "textures/grass_top.png",
                "textures/grass_side.png",
                "textures/dirt.png",
            )),
            ..Default::default()
        },
    );
    let def = block_def_from_schema(&path, &block);
    assert_eq!(def.texture_top, "textures/grass_top.png");
    assert_eq!(def.texture_side, "textures/grass_side.png");
}
