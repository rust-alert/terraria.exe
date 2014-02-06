//! 配方注册。vanilla 与 mod 共用 builder，不维护平行 `RECIPES` 常量数组。

use crate::content::ContentRegistry;
use crate::ItemId;

/// 制作站需求。后续可扩展为任意 `BlockId` 列表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeStation {
    /// 徒手。
    Hand,
    /// 须靠近工作台。
    Workbench,
    /// 须靠近熔炉。
    Furnace,
}

/// 一条已登记配方。
#[derive(Debug, Clone)]
pub struct RecipeDef {
    /// 稳定键。
    pub key: String,
    /// 制作站。
    pub station: RecipeStation,
    /// 材料。
    pub inputs: Vec<(ItemId, u32)>,
    /// 产物。
    pub output: ItemId,
    /// 产物数量。
    pub output_count: u32,
    /// 界面短标签。
    pub label: String,
}

/// 配方登记进行中。
pub struct RecipeRegistration<'a> {
    registry: &'a mut ContentRegistry,
    def: RecipeDef,
}

impl ContentRegistry {
    /// 开始登记一条配方。
    pub fn recipe(&mut self, output: ItemId, count: u32) -> RecipeRegistration<'_> {
        RecipeRegistration {
            registry: self,
            def: RecipeDef {
                key: format!("recipe:{}:{}", output.0, count),
                station: RecipeStation::Hand,
                inputs: Vec::new(),
                output,
                output_count: count.max(1),
                label: String::new(),
            },
        }
    }

    /// 已冻结的配方表。
    pub fn recipes(&self) -> &[RecipeDef] {
        &self.recipes
    }

    pub(crate) fn push_recipe(&mut self, def: RecipeDef) -> Result<(), String> {
        self.ensure_writable()?;
        if self.recipes.iter().any(|r| r.key == def.key) {
            return Err(format!("配方 key 重复：{}", def.key));
        }
        self.recipes.push(def);
        Ok(())
    }
}

impl RecipeRegistration<'_> {
    /// 稳定键。
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.def.key = key.into();
        self
    }

    /// 界面标签。
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.def.label = label.into();
        self
    }

    /// 制作站。
    pub fn station(mut self, station: RecipeStation) -> Self {
        self.def.station = station;
        self
    }

    /// 一种材料。
    pub fn ingredient(mut self, item: ItemId, count: u32) -> Self {
        if count > 0 {
            self.def.inputs.push((item, count));
        }
        self
    }

    /// 写入注册表。
    pub fn register(self) -> Result<(), String> {
        if self.def.label.is_empty() {
            return Err(format!("配方 {} 缺少标签", self.def.key));
        }
        if self.def.inputs.is_empty() {
            return Err(format!("配方 {} 缺少材料", self.def.key));
        }
        self.registry.push_recipe(self.def)
    }
}
