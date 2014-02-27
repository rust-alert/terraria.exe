//! 内容模块：vanilla 与 mod 走同一注册入口。
//!
//! 玩法定义由实现 [`ContentModule`] 的代码在启动时写入 [`ContentRegistry`]。
//! 仓库不携带从正版安装导出的表文件。素材只存逻辑键，运行时再从用户安装解析。

use std::collections::HashMap;

use crate::content::{BlockDef, ContentRegistry, ItemDef, NpcDef, WallDef, install, is_installed};
use crate::tile_sets::{TileSets, install_tile_sets, try_tile_sets};
use crate::weapon::WeaponStats;
use crate::{BlockId, ItemId, NpcId, WallId};

/// 一个内容包。vanilla 与 mod 都实现本 trait。
pub trait ContentModule {
    /// 诊断用短名。同一轮启动里必须唯一。
    fn name(&self) -> &str;

    /// 必须先完成登记的模块名。缺依赖或成环时启动失败。
    fn depends_on(&self) -> &[&str] {
        &[]
    }

    /// 把本模块的定义写入注册表。不得假设未声明依赖的模块已写完。
    fn register(&self, registry: &mut ContentRegistry) -> Result<(), String>;
}

/// 运行一组内容模块，冻结注册表，并派生 [`TileSets`]。
///
/// 进程内只允许成功启动一次。重复调用在已安装时直接返回。
pub fn boot_content_modules(modules: &[&dyn ContentModule]) -> Result<(), String> {
    if is_installed() {
        return Ok(());
    }
    let mut registry = ContentRegistry::new();
    for &index in module_order(modules)?.iter() {
        let module = modules[index];
        registry.note_module(module.name());
        module
            .register(&mut registry)
            .map_err(|e| format!("{}: {e}", module.name()))?;
    }
    let has_tiles = registry.iter_blocks().next().is_some();
    let sets = tile_sets_from_registry(&registry);
    registry.seal();
    install(registry);
    // 空模块启动不装属性表，避免把未登记 id 一律判成非实心。有登记时才派生。
    if has_tiles && try_tile_sets().is_none() {
        install_tile_sets(sets).map_err(|_| "物块属性表已安装".to_string())?;
    }
    Ok(())
}

/// 按依赖把模块排成登记顺序。无依赖时保持输入顺序。
fn module_order(modules: &[&dyn ContentModule]) -> Result<Vec<usize>, String> {
    let n = modules.len();
    let mut by_name: HashMap<&str, usize> = HashMap::with_capacity(n);
    for (index, module) in modules.iter().enumerate() {
        if by_name.insert(module.name(), index).is_some() {
            return Err(format!("内容模块重名：{}", module.name()));
        }
    }
    let mut next: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut waiting = vec![0usize; n];
    for (index, module) in modules.iter().enumerate() {
        for dep in module.depends_on() {
            let Some(&dep_index) = by_name.get(dep) else {
                return Err(format!("模块 {} 依赖未提供的 {}", module.name(), dep));
            };
            if dep_index == index {
                return Err(format!("模块 {} 不能依赖自己", module.name()));
            }
            next[dep_index].push(index);
            waiting[index] += 1;
        }
    }
    let mut ready: Vec<usize> = (0..n).filter(|&index| waiting[index] == 0).collect();
    ready.sort_unstable();
    let mut order = Vec::with_capacity(n);
    while !ready.is_empty() {
        let index = ready.remove(0);
        order.push(index);
        let mut unlocked = Vec::new();
        for &child in &next[index] {
            waiting[child] -= 1;
            if waiting[child] == 0 {
                unlocked.push(child);
            }
        }
        ready.extend(unlocked);
        ready.sort_unstable();
        ready.dedup();
    }
    if order.len() != n {
        return Err("内容模块依赖成环".into());
    }
    Ok(order)
}

/// 从已登记方块派生启动期属性数组。未登记的类型保持 `None`。
pub fn tile_sets_from_registry(registry: &ContentRegistry) -> TileSets {
    let mut sets = TileSets::new();
    for (id, def) in registry.iter_blocks() {
        sets.set_solid(id.0, def.solid);
        sets.set_solid_top(id.0, def.is_platform);
        sets.set_frame_important(id.0, def.frame_important);
        sets.set_merge_dirt(id.0, def.framed_terrain);
    }
    sets
}

/// 按原版类型 id 登记一种物块。
pub struct TileRegistration<'a> {
    registry: &'a mut ContentRegistry,
    id: u32,
    def: BlockDef,
}

impl ContentRegistry {
    /// 开始登记 `id` 号物块。数值必须是内容身份，禁止另造夹具编号。
    pub fn tile(&mut self, id: u32) -> TileRegistration<'_> {
        TileRegistration {
            registry: self,
            id,
            def: BlockDef {
                key: format!("unnamed:{id}"),
                name: String::new(),
                solid: false,
                blocks_motion: false,
                ladder: false,
                replaceable: false,
                max_hp: 0,
                light_radius: 0,
                mine_power_need: None,
                drop: None,
                color: [0.5, 0.5, 0.5, 1.0],
                texture: String::new(),
                texture_top: String::new(),
                texture_side: String::new(),
                texture_bottom: String::new(),
                is_tree: false,
                frame_important: false,
                // 身份对齐后贴图文件号与类型 id 相同。
                texture_file: Some(id),
                is_platform: false,
                is_fluid: false,
                framed_terrain: false,
                house_space: false,
                house_furniture: false,
            },
        }
    }
}

impl TileRegistration<'_> {
    /// 稳定键，如 `terraria:dirt`。
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.def.key = key.into();
        self
    }

    /// 显示名（可先放符号，再由本地化解析）。
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.def.name = name.into();
        self
    }

    /// 是否实心。
    pub fn solid(mut self, value: bool) -> Self {
        self.def.solid = value;
        self.def.blocks_motion = value;
        self
    }

    /// 是否挡移动（可与 solid 不同，例如某些机关）。
    pub fn blocks_motion(mut self, value: bool) -> Self {
        self.def.blocks_motion = value;
        self
    }

    /// 只挡自上而下的脚。
    pub fn solid_top(mut self, value: bool) -> Self {
        self.def.is_platform = value;
        if value {
            self.def.solid = true;
            self.def.blocks_motion = true;
        }
        self
    }

    /// 放置时是否自带帧。
    pub fn frame_important(mut self, value: bool) -> Self {
        self.def.frame_important = value;
        self
    }

    /// 是否自然树干一类。
    pub fn tree(mut self, value: bool) -> Self {
        self.def.is_tree = value;
        self
    }

    /// 是否液体占位。
    pub fn fluid(mut self, value: bool) -> Self {
        self.def.is_fluid = value;
        self
    }

    /// 是否可攀爬。
    pub fn ladder(mut self, value: bool) -> Self {
        self.def.ladder = value;
        self
    }

    /// 是否可被直接替换。
    pub fn replaceable(mut self, value: bool) -> Self {
        self.def.replaceable = value;
        self
    }

    /// 是否计入房屋室内面积。
    pub fn house_space(mut self, value: bool) -> Self {
        self.def.house_space = value;
        self
    }

    /// 是否满足房屋家具要求。
    pub fn house_furniture(mut self, value: bool) -> Self {
        self.def.house_furniture = value;
        self
    }

    /// 是否按邻格与泥土混接的地形。
    pub fn merge_dirt(mut self, value: bool) -> Self {
        self.def.framed_terrain = value;
        self
    }

    /// 最大耐久。
    pub fn max_hp(mut self, value: u16) -> Self {
        self.def.max_hp = value;
        self
    }

    /// 光照半径。
    pub fn light_radius(mut self, value: i32) -> Self {
        self.def.light_radius = value;
        self
    }

    /// 开采所需镐力。
    pub fn mine_power(mut self, value: u16) -> Self {
        self.def.mine_power_need = Some(value);
        self
    }

    /// 破坏掉落的物品类型 id。
    pub fn drop(mut self, item: ItemId) -> Self {
        self.def.drop = Some(item);
        self
    }

    /// `Tiles_N` 文件编号。身份已对齐时通常等于类型 id。
    pub fn tiles_file(mut self, file_id: Option<u32>) -> Self {
        self.def.texture_file = file_id;
        self
    }

    /// 写入注册表。
    pub fn register(self) -> Result<(), String> {
        if self.def.name.is_empty() {
            return Err(format!("物块 {} 缺少显示名", self.id));
        }
        self.registry
            .register_block_at(BlockId(self.id), self.def)
    }
}

/// 按类型 id 登记一种物品。
pub struct ItemRegistration<'a> {
    registry: &'a mut ContentRegistry,
    id: u32,
    def: ItemDef,
}

impl ContentRegistry {
    /// 开始登记 `id` 号物品。
    pub fn item_entry(&mut self, id: u32) -> ItemRegistration<'_> {
        ItemRegistration {
            registry: self,
            id,
            def: ItemDef {
                key: format!("unnamed:{id}"),
                name: String::new(),
                places: None,
                wall: None,
                heal: None,
                mine_power: None,
                max_durability: 0,
                weapon: None,
                color: [0.5, 0.5, 0.5],
                in_palette: true,
                texture: String::new(),
                texture_file: None,
                bag_bonus_slots: 0,
                is_grapple: false,
                is_hammer: false,
                is_accessory: false,
            },
        }
    }

    /// 开始登记 `id` 号墙。
    pub fn wall_entry(&mut self, id: u8) -> WallRegistration<'_> {
        WallRegistration {
            registry: self,
            id,
            def: WallDef {
                key: format!("unnamed_wall:{id}"),
                name: String::new(),
                max_hp: 40,
                drop: None,
                texture_file: Some(u32::from(id)),
            },
        }
    }

    /// 开始登记 `id` 号 NPC。
    pub fn npc_entry(&mut self, id: u32) -> NpcRegistration<'_> {
        NpcRegistration {
            registry: self,
            id,
            def: NpcDef {
                key: format!("unnamed_npc:{id}"),
                name: String::new(),
                texture_file: Some(id),
            },
        }
    }
}

impl ItemRegistration<'_> {
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.def.key = key.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.def.name = name.into();
        self
    }

    pub fn places(mut self, block: BlockId) -> Self {
        self.def.places = Some(block);
        self
    }

    pub fn wall(mut self, wall: WallId) -> Self {
        self.def.wall = Some(wall);
        self
    }

    pub fn item_file(mut self, file_id: u32) -> Self {
        self.def.texture_file = Some(file_id);
        self
    }

    pub fn heal(mut self, amount: f32) -> Self {
        self.def.heal = Some(amount);
        self
    }

    pub fn mine_power(mut self, power: u16) -> Self {
        self.def.mine_power = Some(power);
        self
    }

    pub fn durability(mut self, value: u16) -> Self {
        self.def.max_durability = value;
        self
    }

    pub fn bag_slots(mut self, extra: u32) -> Self {
        self.def.bag_bonus_slots = extra;
        self
    }

    pub fn grapple(mut self) -> Self {
        self.def.is_grapple = true;
        self
    }

    pub fn hammer(mut self) -> Self {
        self.def.is_hammer = true;
        self
    }

    pub fn accessory(mut self) -> Self {
        self.def.is_accessory = true;
        self
    }

    pub fn weapon(mut self, stats: WeaponStats) -> Self {
        self.def.weapon = Some(stats);
        self
    }

    pub fn register(self) -> Result<(), String> {
        if self.def.name.is_empty() {
            return Err(format!("物品 {} 缺少显示名", self.id));
        }
        self.registry
            .register_item_at(ItemId(self.id), self.def)
    }
}

/// 墙登记进行中。
pub struct WallRegistration<'a> {
    registry: &'a mut ContentRegistry,
    id: u8,
    def: WallDef,
}

impl WallRegistration<'_> {
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.def.key = key.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.def.name = name.into();
        self
    }

    pub fn max_hp(mut self, value: u16) -> Self {
        self.def.max_hp = value;
        self
    }

    pub fn drop(mut self, item: ItemId) -> Self {
        self.def.drop = Some(item);
        self
    }

    pub fn wall_file(mut self, file_id: u32) -> Self {
        self.def.texture_file = Some(file_id);
        self
    }

    pub fn register(self) -> Result<(), String> {
        if self.def.name.is_empty() {
            return Err(format!("墙 {} 缺少显示名", self.id));
        }
        self.registry
            .register_wall_at(WallId(self.id), self.def)
    }
}

/// NPC 登记进行中。
pub struct NpcRegistration<'a> {
    registry: &'a mut ContentRegistry,
    id: u32,
    def: NpcDef,
}

impl NpcRegistration<'_> {
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.def.key = key.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.def.name = name.into();
        self
    }

    pub fn npc_file(mut self, file_id: u32) -> Self {
        self.def.texture_file = Some(file_id);
        self
    }

    pub fn register(self) -> Result<(), String> {
        if self.def.name.is_empty() {
            return Err(format!("NPC {} 缺少显示名", self.id));
        }
        self.registry
            .register_npc_at(NpcId(self.id), self.def)
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentModule, boot_content_modules};
    use crate::content::ContentRegistry;
    use crate::tile_sets::try_tile_sets;
    use crate::try_content;

    struct ExampleMod;

    impl ContentModule for ExampleMod {
        fn name(&self) -> &str {
            "example"
        }

        fn register(&self, registry: &mut ContentRegistry) -> Result<(), String> {
            registry
                .tile(400)
                .key("example:demo")
                .name("示例")
                .solid(true)
                .max_hp(10)
                .register()?;
            Ok(())
        }
    }

    #[test]
    fn modules_share_one_registry_and_tile_sets() {
        // 若其它测试已安装则跳过写全局，只测派生函数与指纹。
        if try_content().is_some() {
            let mut reg = ContentRegistry::new();
            reg.note_module("example");
            ExampleMod.register(&mut reg).unwrap();
            let sets = super::tile_sets_from_registry(&reg);
            assert_eq!(sets.solid(400), Some(true));
            assert_eq!(sets.solid(1), None);
            reg.seal();
            assert_ne!(reg.content_hash(), 0);
            return;
        }
        boot_content_modules(&[&ExampleMod]).unwrap();
        let c = try_content().expect("content");
        assert!(c.block(crate::BlockId(400)).is_some());
        assert_ne!(c.content_hash(), 0);
        assert!(c.modules().iter().any(|m| m == "example"));
        let sets = try_tile_sets().expect("tile sets");
        assert_eq!(sets.solid(400), Some(true));
        assert_eq!(sets.solid(999), None);
    }

    struct NamedMod {
        name: &'static str,
        deps: &'static [&'static str],
    }

    impl ContentModule for NamedMod {
        fn name(&self) -> &str {
            self.name
        }

        fn depends_on(&self) -> &[&str] {
            self.deps
        }

        fn register(&self, _registry: &mut ContentRegistry) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn dependencies_register_before_dependents() {
        let child = NamedMod {
            name: "child",
            deps: &["base"],
        };
        let base = NamedMod {
            name: "base",
            deps: &[],
        };
        let mods: [&dyn ContentModule; 2] = [&child, &base];
        let order = super::module_order(&mods).unwrap();
        assert_eq!(mods[order[0]].name(), "base");
        assert_eq!(mods[order[1]].name(), "child");
    }

    #[test]
    fn missing_dependency_is_rejected() {
        let only = NamedMod {
            name: "only",
            deps: &["missing"],
        };
        let mods: [&dyn ContentModule; 1] = [&only];
        let err = super::module_order(&mods).unwrap_err();
        assert!(err.contains("missing"));
    }

    #[test]
    fn cyclic_dependencies_are_rejected() {
        let a = NamedMod {
            name: "a",
            deps: &["b"],
        };
        let b = NamedMod {
            name: "b",
            deps: &["a"],
        };
        let mods: [&dyn ContentModule; 2] = [&a, &b];
        let err = super::module_order(&mods).unwrap_err();
        assert!(err.contains("成环"));
    }

    #[test]
    fn duplicate_module_names_are_rejected() {
        let a = NamedMod {
            name: "same",
            deps: &[],
        };
        let b = NamedMod {
            name: "same",
            deps: &[],
        };
        let mods: [&dyn ContentModule; 2] = [&a, &b];
        let err = super::module_order(&mods).unwrap_err();
        assert!(err.contains("重名"));
    }

    #[test]
    fn overlay_changes_one_field_and_rejects_a_second_write() {
        let mut reg = ContentRegistry::new();
        reg.tile(1)
            .key("terraria:stone")
            .name("石头")
            .solid(true)
            .max_hp(100)
            .register()
            .unwrap();
        reg.overlay_tile("terraria:stone")
            .max_hp(80)
            .apply()
            .unwrap();
        let stone = reg.block(crate::BlockId(1)).unwrap();
        assert_eq!(stone.max_hp, 80);
        assert!(stone.solid);
        assert_eq!(stone.name, "石头");
        let err = reg
            .overlay_tile("terraria:stone")
            .max_hp(1)
            .apply()
            .unwrap_err();
        assert!(err.contains("覆盖冲突"));
        let missing = reg.overlay_tile("missing").name("x").apply().unwrap_err();
        assert!(missing.contains("不存在"));
    }
}
