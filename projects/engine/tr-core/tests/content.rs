use tr_core::{BlockDef, BlockFaceKind, BlockId, ContentRegistry, ItemId};

#[test]
fn register_block_and_faces() {
    let mut reg = ContentRegistry::new();
    reg.register_block_at(
        BlockId(1),
        BlockDef {
            key: "terraria:dirt".into(),
            name: "泥土".into(),
            solid: true,
            blocks_motion: true,
            ladder: false,
            replaceable: false,
            max_hp: 50,
            light_radius: 0,
            mine_power_need: None,
            drop: Some(ItemId(1)),
            color: [0.45, 0.3, 0.18, 1.0],
            texture: "textures/dirt.png".into(),
            texture_top: String::new(),
            texture_side: String::new(),
            texture_bottom: String::new(),
        },
    )
    .unwrap();
    reg.set_block_faces(
        "terraria:dirt",
        "textures/a.png".into(),
        "textures/b.png".into(),
        "textures/c.png".into(),
    )
    .unwrap();
    let d = reg.block(BlockId(1)).unwrap();
    assert_eq!(d.texture_for(BlockFaceKind::Top), "textures/a.png");
}
