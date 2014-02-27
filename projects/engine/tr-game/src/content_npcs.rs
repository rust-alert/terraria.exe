//! 当前会用到的 NPC，按公开类型 id 登记贴图文件号。

use tr_core::{ContentModule, ContentRegistry, NpcId};

/// 随客户端发布的 NPC 登记。
pub struct VanillaNpcs;

impl ContentModule for VanillaNpcs {
    fn name(&self) -> &str {
        "vanilla_npcs"
    }

    fn register(&self, registry: &mut ContentRegistry) -> Result<(), String> {
        let rows: &[(u32, &str, &str)] = &[
            (NpcId::BLUE_SLIME.0, "terraria:blue_slime", "蓝史莱姆"),
            (NpcId::DEMON_EYE.0, "terraria:demon_eye", "恶魔眼"),
            (NpcId::ZOMBIE.0, "terraria:zombie", "僵尸"),
            (NpcId::MERCHANT.0, "terraria:merchant", "商人"),
            (NpcId::GUIDE.0, "terraria:guide", "向导"),
        ];
        for &(id, key, name) in rows {
            registry
                .npc_entry(id)
                .key(key)
                .name(name)
                .npc_file(id)
                .register()?;
        }
        Ok(())
    }
}
