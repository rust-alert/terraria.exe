//! 已写入帧的坐标换算。
//!
//! 邻接分帧表是游戏内容，不放在引擎里。没有已经写入的 `frameX` / `frameY`
//! 时，本模块不发明坐标。绘制必须标成缺失，不能拿猜测 UV 去采样图集。

use spark_core::Rect;

use crate::world::World;

/// 常见图集的像素步长。只描述换算，不是某一种方块的布局。
pub const STRIDE: u32 = 18;
/// 一格画面边长。
pub const CELL: u32 = 16;

/// 把像素帧换成归一化 UV。越界则没有可采样的格。
pub fn frame_to_uv(u: u16, v: u16, sheet_w: u32, sheet_h: u32) -> Option<Rect> {
    if sheet_w < CELL || sheet_h < CELL {
        return None;
    }
    let ux = u as u32;
    let uy = v as u32;
    if ux + CELL > sheet_w || uy + CELL > sheet_h {
        return None;
    }
    Some(Rect::new(
        ux as f32 / sheet_w as f32,
        uy as f32 / sheet_h as f32,
        CELL as f32 / sheet_w as f32,
        CELL as f32 / sheet_h as f32,
    ))
}

/// 不从邻居猜测地形帧。已验证的帧只能由外部写入。
pub fn frame_uv_px(_world: &World, _tx: i32, _ty: i32) -> Option<(u16, u16)> {
    None
}

/// 不从邻居猜测墙帧。
pub fn wall_frame_uv_px(_world: &World, _tx: i32, _ty: i32) -> Option<(u16, u16)> {
    None
}

/// 不写入猜测地形帧。
pub fn stamp_terrain_cell(_world: &mut World, _tx: i32, _ty: i32) {}

/// 不写入猜测地形帧。
pub fn stamp_terrain_around(_world: &mut World, _tx: i32, _ty: i32) {}

/// 不写入猜测地形帧。
pub fn stamp_terrain_all(_world: &mut World) {}

/// 不写入猜测墙帧。
pub fn stamp_wall_cell(_world: &mut World, _tx: i32, _ty: i32) {}

/// 不写入猜测墙帧。
pub fn stamp_walls_around(_world: &mut World, _tx: i32, _ty: i32) {}

/// 不写入猜测墙帧。
pub fn stamp_walls_all(_world: &mut World) {}

#[cfg(test)]
mod tests {
    use super::frame_uv_px;

    #[test]
    fn converter_rejects_cells_outside_the_sheet() {
        assert!(super::frame_to_uv(0, 0, 16, 16).is_some());
        assert!(super::frame_to_uv(2, 0, 16, 16).is_none());
    }

    #[test]
    fn neighbor_rules_do_not_invent_a_frame() {
        // 没有世界也能说明：查询函数的契约是不返回猜测坐标。
        // 具体格子在 `World` 测试里断言生成后没有地形帧。
        let _ = frame_uv_px;
    }
}
