//! 制作查询。配方行来自内容注册表，不在本文件维护平行数组。

use tr_core::{RecipeDef, RecipeStation, try_content};

/// 制作站（与 [`RecipeStation`] 同义，留给界面层使用）。
pub type CraftStation = RecipeStation;

/// 当前会话已登记的配方。未启动内容图时为空。
pub fn recipes() -> &'static [RecipeDef] {
    match try_content() {
        Some(c) => c.recipes(),
        None => &[],
    }
}
