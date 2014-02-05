//! 按类型 id 索引的物块属性。
//!
//! 格子上只存类型号。空气是格子未激活，不是一种方块。
//! 固体、只挡脚、是否带帧，都是 `属性[type]`。宿主在启动时写入。
//! 本模块不携带任何方块行、名字或贴图。

use std::sync::OnceLock;

static TILE_SETS: OnceLock<TileSets> = OnceLock::new();

/// 某一类属性。下标是类型 id。`None` 表示宿主还没写这个 id。
#[derive(Debug, Clone, Default)]
struct FlagSet {
    values: Vec<Option<bool>>,
}

impl FlagSet {
    fn set(&mut self, id: u32, value: bool) {
        let i = id as usize;
        if self.values.len() <= i {
            self.values.resize(i + 1, None);
        }
        self.values[i] = Some(value);
    }

    fn get(&self, id: u32) -> Option<bool> {
        self.values.get(id as usize).copied().flatten()
    }
}

/// 物块属性数组。对应启动时填好的 `tileSolid[type]`、`tileFrameImportant[type]`。
#[derive(Debug, Clone, Default)]
pub struct TileSets {
    solid: FlagSet,
    /// 只挡自上而下的脚（平台）。
    solid_top: FlagSet,
    frame_important: FlagSet,
    /// 与泥土按邻格混接的地形。
    merge_dirt: FlagSet,
}

impl TileSets {
    /// 空表。未写入的 id 查询得到 `None`。
    pub fn new() -> Self {
        Self::default()
    }

    /// 写入 `tileSolid[id]`。
    pub fn set_solid(&mut self, id: u32, value: bool) {
        self.solid.set(id, value);
    }

    /// 写入只挡脚。
    pub fn set_solid_top(&mut self, id: u32, value: bool) {
        self.solid_top.set(id, value);
    }

    /// 写入该类型放置时是否自带帧。
    pub fn set_frame_important(&mut self, id: u32, value: bool) {
        self.frame_important.set(id, value);
    }

    /// 写入是否按邻格与泥土混接。
    pub fn set_merge_dirt(&mut self, id: u32, value: bool) {
        self.merge_dirt.set(id, value);
    }

    /// `tileSolid[id]`。宿主没写过则 `None`。
    pub fn solid(&self, id: u32) -> Option<bool> {
        self.solid.get(id)
    }

    /// 只挡脚。
    pub fn solid_top(&self, id: u32) -> Option<bool> {
        self.solid_top.get(id)
    }

    /// 是否带帧。
    pub fn frame_important(&self, id: u32) -> Option<bool> {
        self.frame_important.get(id)
    }

    /// 是否地形混接。
    pub fn merge_dirt(&self, id: u32) -> Option<bool> {
        self.merge_dirt.get(id)
    }
}

/// 安装属性数组。只能安装一次。
pub fn install_tile_sets(sets: TileSets) -> Result<(), TileSets> {
    TILE_SETS.set(sets)
}

/// 已安装的属性数组。
pub fn try_tile_sets() -> Option<&'static TileSets> {
    TILE_SETS.get()
}

#[cfg(test)]
mod tests {
    use super::TileSets;

    #[test]
    fn unset_type_is_not_a_row_in_the_engine() {
        let mut sets = TileSets::new();
        sets.set_solid(0, true);
        sets.set_frame_important(5, true);
        assert_eq!(sets.solid(0), Some(true));
        assert_eq!(sets.solid(400), None);
        assert_eq!(sets.frame_important(5), Some(true));
        assert_eq!(sets.frame_important(0), None);
    }
}
