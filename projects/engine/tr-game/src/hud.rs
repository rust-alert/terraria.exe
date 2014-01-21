//! HUD：原版式左上快捷栏、右上生命心 / 魔力星、迷你地图条与面板。

use spark_core::{Color, Rect, Vec2};
use spark_renderer::DrawList;
use spark_widget::{label, panel};
use tr_core::ItemId;

use crate::craft::{CraftStation, RECIPES};
use crate::world::{TILE, WORLD_H, WORLD_W, wrap_tx};

use crate::app::TerrariaApp;

/// 底栏与数字键对齐的前 N 格。
pub(crate) const HOTBAR_SLOTS: usize = 10;

/// HUD 指针命中（用于点击与遮挡世界交互）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudPointer {
    None,
    /// 点在信息面板等非按钮区域：吞掉点击，不挖世界。
    Chrome,
    Hotbar(usize),
    BagSlot(usize),
    /// 木箱列表行。
    ChestRow(usize),
    /// 背包内徒手配方行 → `RECIPES` 下标。
    BagCraft(usize),
    CraftRow(usize),
    /// 商人商店货架行 → `MERCHANT_OFFERS` 下标。
    ShopRow(usize),
    RailCraft,
    RailBag,
    RailMap,
    RailSettings,
    QuickMap,
    QuickBag,
}

const PANEL: Color = crate::palette::HUD_PANEL;
const PANEL_EDGE: Color = crate::palette::HUD_PANEL_EDGE;
const TEXT_MAIN: Color = crate::palette::HUD_TEXT_MAIN;
const TEXT_DIM: Color = crate::palette::HUD_TEXT_DIM;
const ACCENT: Color = crate::palette::HUD_ACCENT;

/// 背包网格列数。
const BAG_COLS: usize = 7;
const BAG_SLOT: f32 = 52.0;
const BAG_GAP: f32 = 6.0;

fn item_swatch(id: ItemId) -> Color {
    match id {
        ItemId::DIRT => Color::rgb(0.55, 0.38, 0.22),
        ItemId::STONE => Color::rgb(0.55, 0.58, 0.62),
        ItemId::WOOD => Color::rgb(0.7, 0.48, 0.25),
        ItemId::WORKBENCH => Color::rgb(0.8, 0.55, 0.3),
        ItemId::SAPLING => Color::rgb(0.4, 0.85, 0.35),
        ItemId::WOOD_PICK => Color::rgb(0.65, 0.5, 0.35),
        ItemId::STONE_PICK => Color::rgb(0.7, 0.72, 0.78),
        ItemId::WOOD_SWORD => Color::rgb(0.85, 0.75, 0.4),
        ItemId::TORCH => Color::rgb(1.0, 0.7, 0.25),
        ItemId::GEL => Color::rgb(0.75, 0.35, 0.85),
        ItemId::PLATFORM => Color::rgb(0.7, 0.48, 0.25),
        ItemId::CHEST => Color::rgb(0.85, 0.6, 0.28),
        ItemId::LADDER => Color::rgb(0.65, 0.42, 0.2),
        ItemId::WOOD_ARMOR => Color::rgb(0.55, 0.4, 0.25),
        ItemId::GEL_STAFF => Color::rgb(0.45, 0.7, 1.0),
        ItemId::WOOD_BOW => Color::rgb(0.6, 0.45, 0.3),
        ItemId::GRAPPLE => Color::rgb(0.7, 0.72, 0.78),
        ItemId::CLOUD_BOTTLE => Color::rgb(0.55, 0.78, 0.95),
        ItemId::ROPE => Color::rgb(0.72, 0.55, 0.28),
        ItemId::WOOD_WALL => Color::rgb(0.62, 0.42, 0.22),
        ItemId::STONE_WALL => Color::rgb(0.48, 0.5, 0.55),
        ItemId::COPPER_COIN => Color::rgb(0.85, 0.55, 0.28),
        _ => Color::rgb(0.5, 0.55, 0.6),
    }
}

fn clock_from_day_t(day_t: f32) -> (u32, u32) {
    let phase = (day_t / 180.0).rem_euclid(1.0);
    let hours = phase * 24.0;
    let h = hours.floor() as u32;
    let m = ((hours - h as f32) * 60.0).floor() as u32;
    (h % 24, m % 60)
}

fn hotbar_geom(screen_w: f32, _screen_h: f32) -> (f32, f32, f32, f32, usize) {
    let slot = 44.0;
    let gap = 4.0;
    let n = HOTBAR_SLOTS;
    let _ = screen_w;
    // 原版快捷栏在左上。
    let base_x = 20.0;
    let base_y = 20.0;
    (base_x, base_y, slot, gap, n)
}

fn vitals_rect(screen_w: f32) -> Rect {
    Rect::new(screen_w - 280.0, 12.0, 264.0, 120.0)
}

fn minimap_chip_rect() -> Rect {
    Rect::new(72.0, 48.0, 150.0, 64.0)
}

fn craft_panel_rect(screen_w: f32, recipe_n: usize) -> Rect {
    let visible = recipe_n.min(12) as f32;
    Rect::new(
        screen_w * 0.5 - 200.0,
        80.0,
        400.0,
        56.0 + visible * 36.0 + 28.0,
    )
}

fn bag_rows(slot_count: usize) -> usize {
    slot_count.max(1).div_ceil(BAG_COLS)
}

fn hand_recipe_indices() -> Vec<usize> {
    RECIPES
        .iter()
        .enumerate()
        .filter(|(_, r)| r.station == CraftStation::Hand)
        .map(|(i, _)| i)
        .collect()
}

fn bag_panel_rect(screen_w: f32, slot_count: usize) -> Rect {
    let rows = bag_rows(slot_count) as f32;
    let grid_w = BAG_COLS as f32 * (BAG_SLOT + BAG_GAP) - BAG_GAP;
    let grid_h = rows * (BAG_SLOT + BAG_GAP) - BAG_GAP;
    let hand_n = hand_recipe_indices().len() as f32;
    let craft_h = 36.0 + hand_n * 34.0;
    let w = grid_w + 40.0;
    let h = grid_h + 72.0 + craft_h + 12.0;
    Rect::new(screen_w * 0.5 - w * 0.5, 64.0, w, h.min(640.0))
}

fn bag_slot_rect(screen_w: f32, slot_count: usize, index: usize) -> Rect {
    let box_r = bag_panel_rect(screen_w, slot_count);
    let col = index % BAG_COLS;
    let row = index / BAG_COLS;
    let x0 = box_r.x + 20.0;
    let y0 = box_r.y + 52.0;
    Rect::new(
        x0 + col as f32 * (BAG_SLOT + BAG_GAP),
        y0 + row as f32 * (BAG_SLOT + BAG_GAP),
        BAG_SLOT,
        BAG_SLOT,
    )
}

fn bag_craft_row_rect(screen_w: f32, slot_count: usize, hand_row: usize) -> Rect {
    let box_r = bag_panel_rect(screen_w, slot_count);
    let grid_h = bag_rows(slot_count) as f32 * (BAG_SLOT + BAG_GAP) - BAG_GAP;
    let y0 = box_r.y + 52.0 + grid_h + 40.0;
    Rect::new(
        box_r.x + 16.0,
        y0 + hand_row as f32 * 34.0,
        box_r.w - 32.0,
        30.0,
    )
}

fn map_panel_rect(screen_w: f32, screen_h: f32) -> Rect {
    let w = (screen_w * 0.62).clamp(420.0, 720.0);
    let h = (screen_h * 0.58).clamp(320.0, 480.0);
    Rect::new((screen_w - w) * 0.5, (screen_h - h) * 0.5 - 20.0, w, h)
}

fn shop_panel_rect(screen_w: f32) -> Rect {
    let n = crate::shop::MERCHANT_OFFERS.len() as f32;
    Rect::new(screen_w * 0.5 - 220.0, 88.0, 440.0, 64.0 + n * 40.0 + 36.0)
}

fn shop_row_rect(screen_w: f32, i: usize) -> Rect {
    let panel = shop_panel_rect(screen_w);
    Rect::new(
        panel.x + 16.0,
        panel.y + 56.0 + i as f32 * 40.0,
        panel.w - 32.0,
        34.0,
    )
}

fn brand_rect() -> Rect {
    Rect::new(20.0, 72.0, 120.0, 20.0)
}

impl TerrariaApp {
    pub(crate) fn toggle_map(&mut self) {
        self.map_open = !self.map_open;
        if self.map_open {
            self.craft_open = false;
            self.bag_open = false;
            self.shop_open = false;
            self.flush_cursor_to_inv();
        }
    }

    pub(crate) fn toggle_bag(&mut self) {
        self.bag_open = !self.bag_open;
        if self.bag_open {
            self.craft_open = false;
            self.map_open = false;
            self.shop_open = false;
        } else {
            self.flush_cursor_to_inv();
        }
    }

    pub(crate) fn toggle_shop(&mut self) {
        self.shop_open = !self.shop_open;
        if self.shop_open {
            self.craft_open = false;
            self.bag_open = false;
            self.map_open = false;
            self.chest_open = None;
            self.flush_cursor_to_inv();
        }
    }

    /// 指针落在哪块 HUD 上（含仅遮挡的信息面板）。
    pub(crate) fn hud_pointer_at(&self, mx: f32, my: f32) -> HudPointer {
        let p = Vec2::new(mx, my);

        // 可点控件优先于整块 Chrome，否则制作行 / 背包格永远点不到。
        if self.shop_open {
            let panel = shop_panel_rect(self.screen_w);
            if panel.contains(p) {
                for i in 0..crate::shop::MERCHANT_OFFERS.len() {
                    if shop_row_rect(self.screen_w, i).contains(p) {
                        return HudPointer::ShopRow(i);
                    }
                }
                return HudPointer::Chrome;
            }
        }
        if self.craft_open {
            let panel = craft_panel_rect(self.screen_w, RECIPES.len());
            if panel.contains(p) {
                let panel_y = 80.0;
                for i in 0..RECIPES.len().min(12) {
                    let y = panel_y + 52.0 + i as f32 * 36.0;
                    if Rect::new(panel.x + 16.0, y, 368.0, 32.0).contains(p) {
                        return HudPointer::CraftRow(i);
                    }
                }
                return HudPointer::Chrome;
            }
        }
        if self.bag_open {
            let bag_n = self
                .player
                .as_ref()
                .map(|p| p.inv.bag.len())
                .unwrap_or(crate::player::BAG_POCKET_SLOTS);
            let box_r = bag_panel_rect(self.screen_w, bag_n);
            if box_r.contains(p) {
                for i in 0..bag_n {
                    if bag_slot_rect(self.screen_w, bag_n, i).contains(p) {
                        return HudPointer::BagSlot(i);
                    }
                }
                for (row, &ri) in hand_recipe_indices().iter().enumerate() {
                    if bag_craft_row_rect(self.screen_w, bag_n, row).contains(p) {
                        return HudPointer::BagCraft(ri);
                    }
                }
                return HudPointer::Chrome;
            }
        }
        if self.map_open {
            if map_panel_rect(self.screen_w, self.screen_h).contains(p) {
                return HudPointer::Chrome;
            }
        }
        if self.chest_open.is_some() {
            let panel_x = self.screen_w * 0.5 - 200.0;
            let panel_y = 100.0;
            let entries_n = self
                .world
                .as_ref()
                .and_then(|w| {
                    let (cx, cy) = self.chest_open?;
                    w.chests
                        .get(&(cx, cy))
                        .map(|m| m.values().filter(|n| **n > 0).count())
                })
                .unwrap_or(0);
            let rows = entries_n.max(1);
            let h = 56.0 + rows as f32 * 36.0 + 48.0;
            let panel = Rect::new(panel_x, panel_y, 400.0, h);
            if panel.contains(p) {
                for i in 0..entries_n {
                    let row = Rect::new(
                        panel_x + 16.0,
                        panel_y + 56.0 + i as f32 * 36.0,
                        368.0,
                        32.0,
                    );
                    if row.contains(p) {
                        return HudPointer::ChestRow(i);
                    }
                }
                // 空区：当作可存入的虚行
                return HudPointer::ChestRow(entries_n);
            }
        }

        let (base_x, base_y, slot, gap, n) = hotbar_geom(self.screen_w, self.screen_h);
        for i in 0..n {
            let x = base_x + i as f32 * (slot + gap);
            if Rect::new(x - 2.0, base_y - 2.0, slot + 4.0, slot + 4.0).contains(p) {
                return HudPointer::Hotbar(i);
            }
        }

        if brand_rect().contains(p)
            || vitals_rect(self.screen_w).contains(p)
            || minimap_chip_rect().contains(p)
        {
            return HudPointer::Chrome;
        }

        HudPointer::None
    }

    /// `left`：左键；否则右键。`shift`：快速移动。
    pub(crate) fn handle_hud_button(
        &mut self,
        mx: f32,
        my: f32,
        left: bool,
        shift: bool,
    ) -> (bool, Option<String>) {
        let ptr = self.hud_pointer_at(mx, my);
        let container_open = self.bag_open || self.chest_open.is_some();
        match ptr {
            HudPointer::None => (false, None),
            HudPointer::Chrome => (true, None),
            HudPointer::Hotbar(i) => {
                if container_open {
                    self.container_click(crate::container::SlotRef::Hotbar(i), left, shift);
                    (true, None)
                } else if left {
                    if let Some(p) = self.player.as_mut() {
                        p.select_hotbar(i);
                    }
                    (true, None)
                } else {
                    (true, None)
                }
            }
            HudPointer::BagSlot(i) => {
                let equip_msg = if left && !shift {
                    if let Some(p) = self.player.as_mut() {
                        let bag_id = p.inv.bag.get(i).and_then(|s| s.as_ref()).map(|s| s.id);
                        if self.cursor_stack.is_none() {
                            if bag_id == Some(ItemId::WOOD_ARMOR) {
                                return (true, p.inv.equip_armor(ItemId::WOOD_ARMOR));
                            }
                            if bag_id.is_some_and(|id| id.is_accessory()) {
                                return (true, p.inv.equip_accessory(bag_id.unwrap()));
                            }
                        }
                    }
                    None
                } else {
                    None
                };
                self.container_click(crate::container::SlotRef::Bag(i), left, shift);
                (true, equip_msg)
            }
            HudPointer::ChestRow(i) => {
                self.container_click(crate::container::SlotRef::Chest(i), left, shift);
                (true, None)
            }
            HudPointer::BagCraft(i) | HudPointer::CraftRow(i) => {
                let msg = if left {
                    if let (Some(player), Some(world)) = (self.player.as_mut(), self.world.as_mut())
                    {
                        let out = player.try_craft(world, i);
                        if out.as_ref().is_some_and(|m| m.starts_with("制成")) {
                            crate::sfx::craft_ok(&self.sfx, &self.audio);
                        }
                        out
                    } else {
                        None
                    }
                } else {
                    None
                };
                (true, msg)
            }
            HudPointer::ShopRow(i) => {
                let msg = if left {
                    if let Some(player) = self.player.as_mut() {
                        match crate::shop::try_buy(&mut player.inv, i) {
                            Ok(m) => {
                                crate::sfx::coins(&self.sfx, &self.audio);
                                Some(m)
                            }
                            Err(e) => Some(e),
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                (true, msg)
            }
            HudPointer::RailCraft => {
                self.craft_open = !self.craft_open;
                if self.craft_open {
                    self.bag_open = false;
                    self.map_open = false;
                    self.shop_open = false;
                    self.flush_cursor_to_inv();
                }
                (true, None)
            }
            HudPointer::RailBag | HudPointer::QuickBag => {
                self.toggle_bag();
                (true, None)
            }
            HudPointer::RailMap | HudPointer::QuickMap => {
                self.toggle_map();
                (
                    true,
                    Some(if self.map_open {
                        "世界地图 · M / Esc 关闭".into()
                    } else {
                        "已关闭地图".into()
                    }),
                )
            }
            HudPointer::RailSettings => {
                self.screen = crate::app::Screen::Pause;
                (true, None)
            }
        }
    }

    fn container_click(&mut self, slot: crate::container::SlotRef, left: bool, shift: bool) {
        let chest = self.chest_open;
        let Some(player) = self.player.as_mut() else {
            return;
        };
        let Some(world) = self.world.as_mut() else {
            return;
        };
        if shift && left {
            crate::container::click_shift(&mut player.inv, world, chest, slot);
            return;
        }
        if left {
            crate::container::click_left(
                &mut player.inv,
                world,
                chest,
                slot,
                &mut self.cursor_stack,
            );
        } else {
            crate::container::click_right(
                &mut player.inv,
                world,
                chest,
                slot,
                &mut self.cursor_stack,
            );
        }
    }

    pub(crate) fn flush_cursor_to_inv(&mut self) {
        let Some(player) = self.player.as_mut() else {
            return;
        };
        if let Some(left) = crate::container::absorb_cursor(&mut player.inv, &mut self.cursor_stack)
        {
            if let Some(world) = self.world.as_mut() {
                let tx = crate::world::wrap_tx(
                    ((player.x + crate::player::HIT_W * 0.5) / crate::world::TILE).floor() as i32,
                );
                let ty =
                    ((player.y + crate::player::HIT_H * 0.5) / crate::world::TILE).floor() as i32;
                world.spawn_drop_at_tile(tx, ty, left.id, left.count);
            } else {
                self.cursor_stack = Some(left);
            }
        }
    }

    pub(crate) fn paint_hud(&self, draw: &mut DrawList) {
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let Some(world) = self.world.as_ref() else {
            return;
        };

        self.paint_brand(draw);
        self.paint_hotbar(draw, player);
        self.paint_vitals(draw, player);
        self.paint_minimap_chip(draw, player, world);

        if self.toast_t > 0.0 {
            label(
                draw,
                96.0,
                self.screen_h - 168.0,
                13.0,
                Color::rgb(0.95, 0.85, 0.45),
                &self.toast,
            );
        }

        if self.debug_hud {
            self.paint_debug_overlay(draw, player, world);
        }

        if self.craft_open {
            self.paint_craft(draw);
        }
        if self.bag_open {
            self.paint_bag(draw);
        }
        if self.shop_open {
            self.paint_shop(draw);
        }
        if self.map_open {
            self.paint_map(draw);
        }
        if self.chest_open.is_some() {
            self.paint_chest(draw);
        }

        // 光标物品跟鼠标。
        if let Some(stack) = &self.cursor_stack {
            let (mx, my) = self.mouse;
            let id = stack.id;
            if let Some(view) = self.icon_atlas.view(id) {
                draw.tex_rect(
                    view.tex,
                    Rect::new(mx - 14.0, my - 14.0, 28.0, 28.0),
                    view.uv,
                    Color::rgba(1.0, 1.0, 1.0, 0.95),
                );
            } else {
                draw.fill_rect(Rect::new(mx - 14.0, my - 14.0, 28.0, 28.0), item_swatch(id));
            }
            if stack.count > 1 {
                label(
                    draw,
                    mx + 4.0,
                    my + 6.0,
                    12.0,
                    TEXT_MAIN,
                    &format!("{}", stack.count),
                );
            }
        }
    }

    fn paint_brand(&self, draw: &mut DrawList) {
        // 左上留给快捷栏；标题挪到热键下方。
        label(draw, 20.0, 72.0, 14.0, TEXT_DIM, "Terraria");
    }

    fn paint_vitals(&self, draw: &mut DrawList, player: &crate::player::Player) {
        // 原版式：右上生命心 + 右侧魔力星（约 20 HP / 心，20 MP / 星）。
        let heart_n = ((player.max_hp / 20.0).ceil() as i32).clamp(1, 20);
        let filled = (player.hp / 20.0).clamp(0.0, heart_n as f32);
        let star_n = ((player.max_mp / 20.0).ceil() as i32).clamp(0, 20);
        let star_f = (player.mp / 20.0).clamp(0.0, star_n as f32);

        let right = self.screen_w - 16.0;
        let top = 16.0;
        let cols = 10i32;
        let heart_sz = 22.0;
        for i in 0..heart_n {
            let col = i % cols;
            let row = i / cols;
            let x = right - (cols - col) as f32 * 26.0;
            let y = top + row as f32 * 24.0;
            let amt = (filled - i as f32).clamp(0.0, 1.0);
            self.hud_chrome.paint_heart(draw, x, y, heart_sz, amt);
        }
        let mana_sz = 22.0;
        for i in 0..star_n {
            let x = right - 24.0;
            let y = top + 56.0 + i as f32 * 22.0;
            let amt = (star_f - i as f32).clamp(0.0, 1.0);
            self.hud_chrome.paint_mana(draw, x, y, mana_sz, amt);
        }

        // 铜币计数：右上生命心下方。
        let coins = player.inv.get(ItemId::COPPER_COIN);
        let coin_y = top + 52.0 + if star_n > 0 { 0.0 } else { 8.0 };
        if let Some(view) = self.icon_atlas.view(ItemId::COPPER_COIN) {
            draw.tex_rect(
                view.tex,
                Rect::new(right - 120.0, coin_y, 18.0, 18.0),
                view.uv,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
            );
        } else {
            draw.fill_rect(
                Rect::new(right - 120.0, coin_y, 18.0, 18.0),
                item_swatch(ItemId::COPPER_COIN),
            );
        }
        label(
            draw,
            right - 98.0,
            coin_y + 2.0,
            13.0,
            Color::rgb(0.95, 0.78, 0.35),
            &format!("{coins}"),
        );
    }

    fn paint_minimap_chip(
        &self,
        draw: &mut DrawList,
        player: &crate::player::Player,
        world: &crate::world::World,
    ) {
        let w = 150.0;
        let h = 64.0;
        let x = 72.0;
        let y = 48.0;
        panel(
            draw,
            Rect::new(x, y, w, h),
            Color::rgba(PANEL.r, PANEL.g, PANEL.b, 0.55),
        );
        let tx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
        let ty = (player.y / TILE).floor() as i32;
        let (hh, mm) = clock_from_day_t(self.day_t);
        label(
            draw,
            x + 8.0,
            y + 6.0,
            12.0,
            TEXT_DIM,
            &format!("{}  {:02}:{:02}", world.biome_at_x(tx).name(), hh, mm),
        );
        label(
            draw,
            x + 8.0,
            y + 24.0,
            11.0,
            TEXT_DIM,
            &format!("X:{tx}  Y:{ty}"),
        );
        let mx = x + 8.0;
        let my = y + 40.0;
        let mw = w - 16.0;
        let mh = 18.0;
        draw.fill_rect(Rect::new(mx, my, mw, mh), Color::rgb(0.04, 0.06, 0.09));
        let scale = mw / WORLD_W as f32;
        for wx in 0..WORLD_W {
            let hgt = world.surface_at(wx);
            let col_h = ((WORLD_H as f32 - hgt as f32) / WORLD_H as f32 * mh).clamp(1.0, mh);
            let c = match world.biome_at_x(wx) {
                tr_core::BiomeId::Meadow => Color::rgb(0.22, 0.42, 0.22),
                tr_core::BiomeId::Forest => Color::rgb(0.10, 0.32, 0.16),
                tr_core::BiomeId::Desert => Color::rgb(0.55, 0.42, 0.22),
                tr_core::BiomeId::Tundra => Color::rgb(0.35, 0.48, 0.58),
            };
            draw.fill_rect(
                Rect::new(
                    mx + wx as f32 * scale,
                    my + mh - col_h,
                    scale.max(1.0),
                    col_h,
                ),
                c,
            );
        }
        let px = mx + (tx as f32 / WORLD_W as f32) * mw;
        draw.fill_rect(
            Rect::new(px - 1.0, my, 2.0, mh),
            Color::rgb(1.0, 0.85, 0.35),
        );
    }

    fn paint_hotbar(&self, draw: &mut DrawList, player: &crate::player::Player) {
        let (base_x, base_y, slot, gap, n) = hotbar_geom(self.screen_w, self.screen_h);

        for i in 0..n {
            let x = base_x + i as f32 * (slot + gap);
            let selected = player.inv.hotbar_sel % HOTBAR_SLOTS == i;
            self.hud_chrome.paint_slot(draw, x, base_y, slot, selected);
            if let Some(stack) = player.inv.hotbar[i].as_ref() {
                let id = stack.id;
                if let Some(view) = self.icon_atlas.view(id) {
                    draw.tex_rect(
                        view.tex,
                        Rect::new(x + 10.0, base_y + 10.0, 28.0, 28.0),
                        view.uv,
                        Color::rgba(1.0, 1.0, 1.0, 1.0),
                    );
                } else {
                    let icon = item_swatch(id);
                    draw.fill_rect(Rect::new(x + 10.0, base_y + 10.0, 28.0, 28.0), icon);
                }
                if let Some((cur, max)) = stack.tool_dur_left() {
                    let r = (cur as f32 / max.max(1) as f32).clamp(0.0, 1.0);
                    draw.fill_rect(
                        Rect::new(x + 4.0, base_y + slot - 6.0, (slot - 8.0) * r, 3.0),
                        Color::rgb(0.35, 0.85, 0.5),
                    );
                }
                if stack.count > 1 {
                    label(
                        draw,
                        x + 6.0,
                        base_y + slot - 16.0,
                        12.0,
                        TEXT_MAIN,
                        &format!("{}", stack.count),
                    );
                }
            }
            let key = if i + 1 >= 10 { 0 } else { i + 1 };
            label(
                draw,
                x + 4.0,
                base_y + 2.0,
                11.0,
                TEXT_DIM,
                &format!("{key}"),
            );
        }
    }

    fn paint_debug_overlay(
        &self,
        draw: &mut DrawList,
        player: &crate::player::Player,
        world: &crate::world::World,
    ) {
        let tx = wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
        let biome = world.biome_at_x(tx).name();
        let home = if player.home_spawn.is_some() {
            "有"
        } else {
            "无"
        };
        let hand = {
            let sel = player.selected_item();
            if let Some((c, m)) = player.inv.tool_dur_left(sel) {
                format!("{} {}/{}", sel.label(), c, m)
            } else {
                sel.label().to_string()
            }
        };
        panel(
            draw,
            Rect::new(72.0, 64.0, 520.0, 72.0),
            Color::rgba(0.02, 0.04, 0.08, 0.88),
        );
        label(
            draw,
            84.0,
            72.0,
            12.0,
            Color::rgb(0.55, 1.0, 0.7),
            "F3 调试",
        );
        label(
            draw,
            84.0,
            92.0,
            12.0,
            TEXT_DIM,
            &format!(
                "群系:{biome}  家园:{home}  站:{}  怪:{}  甲:{:.0}%  手:{hand}",
                match player.craft_station_now(world) {
                    CraftStation::Hand => "徒手",
                    CraftStation::Workbench => "工作台",
                    CraftStation::Furnace => "熔炉",
                },
                self.enemies.len(),
                player.defense() * 100.0,
            ),
        );
        label(
            draw,
            84.0,
            110.0,
            12.0,
            TEXT_DIM,
            "C 制作  I 包  E 交互  Esc",
        );
    }

    pub(crate) fn paint_chest(&self, draw: &mut DrawList) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let Some((cx, cy)) = self.chest_open else {
            return;
        };
        let panel_x = self.screen_w * 0.5 - 200.0;
        let panel_y = 100.0;
        let entries: Vec<(ItemId, u32)> = world
            .chests
            .get(&(cx, cy))
            .map(|m| {
                m.iter()
                    .filter(|(_, n)| **n > 0)
                    .map(|(a, b)| (*a, *b))
                    .collect()
            })
            .unwrap_or_default();
        let rows = entries.len().max(1);
        let h = 56.0 + rows as f32 * 36.0 + 20.0;
        panel(draw, Rect::new(panel_x, panel_y, 400.0, h), PANEL);
        label(
            draw,
            panel_x + 16.0,
            panel_y + 12.0,
            16.0,
            Color::rgb(0.95, 0.85, 0.55),
            "木箱",
        );
        label(
            draw,
            panel_x + 16.0,
            panel_y + 34.0,
            12.0,
            Color::rgb(0.7, 0.72, 0.8),
            "点击搬运 · 右键拆半 · Shift 快移 · Esc 关闭",
        );
        for (i, (item, n)) in entries.iter().enumerate() {
            let y = panel_y + 56.0 + i as f32 * 36.0;
            draw.fill_rect(
                Rect::new(panel_x + 16.0, y, 368.0, 32.0),
                Color::rgb(0.16, 0.14, 0.10),
            );
            label(
                draw,
                panel_x + 28.0,
                y + 8.0,
                14.0,
                Color::rgb(0.95, 0.92, 0.85),
                &format!("{} ×{}", item.label(), n),
            );
        }
    }

    pub(crate) fn paint_craft(&self, draw: &mut DrawList) {
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let station_now = player.craft_station_now(world);
        let at_wb = matches!(station_now, CraftStation::Workbench);
        let at_fu = matches!(station_now, CraftStation::Furnace);
        let panel_x = self.screen_w * 0.5 - 200.0;
        let panel_y = 80.0;
        let visible = RECIPES.len().min(12);
        panel(
            draw,
            Rect::new(panel_x, panel_y, 400.0, 56.0 + visible as f32 * 36.0 + 28.0),
            PANEL,
        );
        label(
            draw,
            panel_x + 20.0,
            panel_y + 16.0,
            18.0,
            Color::rgb(0.95, 0.95, 1.0),
            match station_now {
                CraftStation::Furnace => "制作 · 熔炉就绪（点击配方）",
                CraftStation::Workbench => "制作 · 工作台就绪（点击配方）",
                CraftStation::Hand => "制作 · 徒手（靠近台/炉解锁）",
            },
        );
        for (i, recipe) in RECIPES.iter().enumerate() {
            let y = panel_y + 52.0 + i as f32 * 36.0;
            if y > self.screen_h - 80.0 {
                break;
            }
            let locked = match recipe.station {
                CraftStation::Hand => false,
                CraftStation::Workbench => !at_wb,
                CraftStation::Furnace => !at_fu,
            };
            let can = !locked && player.inv.can_pay(recipe.inputs);
            let bg = if can {
                Color::rgb(0.14, 0.28, 0.22)
            } else if locked {
                Color::rgb(0.12, 0.12, 0.16)
            } else {
                Color::rgb(0.22, 0.14, 0.14)
            };
            draw.fill_rect(Rect::new(panel_x + 16.0, y, 368.0, 32.0), bg);
            let need: String = recipe
                .inputs
                .iter()
                .map(|(id, n)| format!("{}×{}", id.label(), n))
                .collect::<Vec<_>>()
                .join("+");
            let station = match recipe.station {
                CraftStation::Hand => "徒手",
                CraftStation::Workbench => "台",
                CraftStation::Furnace => "炉",
            };
            let line = format!(
                "{}. {} ←{} [{}]{}",
                i + 1,
                recipe.label,
                need,
                station,
                if locked { "锁" } else { "" }
            );
            label(
                draw,
                panel_x + 24.0,
                y + 8.0,
                13.0,
                Color::rgb(0.92, 0.94, 1.0),
                &line,
            );
        }
    }

    pub(crate) fn paint_shop(&self, draw: &mut DrawList) {
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let box_r = shop_panel_rect(self.screen_w);
        panel(draw, box_r, PANEL);
        draw.fill_rect(Rect::new(box_r.x, box_r.y, box_r.w, 2.0), PANEL_EDGE);
        let coins = player.inv.get(ItemId::COPPER_COIN);
        label(
            draw,
            box_r.x + 20.0,
            box_r.y + 14.0,
            18.0,
            TEXT_MAIN,
            "商人商店",
        );
        label(
            draw,
            box_r.x + 120.0,
            box_r.y + 18.0,
            12.0,
            Color::rgb(0.95, 0.78, 0.35),
            &format!("铜币 {coins} · 点击购买 · Esc"),
        );
        for (i, offer) in crate::shop::MERCHANT_OFFERS.iter().enumerate() {
            let r = shop_row_rect(self.screen_w, i);
            let can = coins >= offer.price;
            draw.fill_rect(
                r,
                if can {
                    Color::rgb(0.14, 0.26, 0.20)
                } else {
                    Color::rgb(0.20, 0.14, 0.14)
                },
            );
            if let Some(view) = self.icon_atlas.view(offer.item) {
                draw.tex_rect(
                    view.tex,
                    Rect::new(r.x + 6.0, r.y + 3.0, 28.0, 28.0),
                    view.uv,
                    Color::rgba(1.0, 1.0, 1.0, 1.0),
                );
            } else {
                draw.fill_rect(
                    Rect::new(r.x + 6.0, r.y + 3.0, 28.0, 28.0),
                    item_swatch(offer.item),
                );
            }
            label(draw, r.x + 42.0, r.y + 8.0, 13.0, TEXT_MAIN, offer.label);
            label(
                draw,
                r.x + r.w - 88.0,
                r.y + 8.0,
                13.0,
                if can {
                    Color::rgb(0.95, 0.78, 0.35)
                } else {
                    Color::rgb(0.75, 0.4, 0.4)
                },
                &format!("{} 铜", offer.price),
            );
        }
    }

    pub(crate) fn paint_bag(&self, draw: &mut DrawList) {
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let bag_n = player.inv.bag.len();
        let box_r = bag_panel_rect(self.screen_w, bag_n);
        panel(draw, box_r, PANEL);
        draw.fill_rect(Rect::new(box_r.x, box_r.y, box_r.w, 2.0), PANEL_EDGE);
        let cap = player.inv.bag_capacity();
        let used = player.inv.used_bag_slots();
        label(
            draw,
            box_r.x + 20.0,
            box_r.y + 14.0,
            18.0,
            TEXT_MAIN,
            "背包",
        );
        label(
            draw,
            box_r.x + 80.0,
            box_r.y + 18.0,
            12.0,
            TEXT_DIM,
            &format!("{used}/{cap} · 拖放/右键拆半/Shift快移 · I/Esc"),
        );
        let gear = {
            let armor = player
                .inv
                .armor
                .as_ref()
                .map(|s| s.id.label())
                .unwrap_or_else(|| "无甲".into());
            let acc = player
                .inv
                .accessory
                .as_ref()
                .map(|s| s.id.label())
                .unwrap_or_else(|| "无饰品".into());
            format!("甲:{armor}  饰:{acc}")
        };
        label(draw, box_r.x + 20.0, box_r.y + 34.0, 11.0, TEXT_DIM, &gear);

        for i in 0..bag_n {
            let slot = bag_slot_rect(self.screen_w, bag_n, i);
            self.hud_chrome
                .paint_slot(draw, slot.x, slot.y, slot.w, false);
            if let Some(stack) = player.inv.bag.get(i).and_then(|s| s.as_ref()) {
                let id = stack.id;
                if let Some(view) = self.icon_atlas.view(id) {
                    draw.tex_rect(
                        view.tex,
                        Rect::new(slot.x + 12.0, slot.y + 10.0, 28.0, 28.0),
                        view.uv,
                        Color::rgba(1.0, 1.0, 1.0, 1.0),
                    );
                } else {
                    let icon = item_swatch(id);
                    draw.fill_rect(Rect::new(slot.x + 12.0, slot.y + 10.0, 28.0, 28.0), icon);
                }
                if let Some((cur, max)) = stack.tool_dur_left() {
                    let r = (cur as f32 / max.max(1) as f32).clamp(0.0, 1.0);
                    draw.fill_rect(
                        Rect::new(slot.x + 4.0, slot.y + slot.h - 6.0, (slot.w - 8.0) * r, 3.0),
                        Color::rgb(0.35, 0.85, 0.5),
                    );
                }
                if stack.count > 0 {
                    label(
                        draw,
                        slot.x + 4.0,
                        slot.y + slot.h - 18.0,
                        12.0,
                        TEXT_MAIN,
                        &format!("{}", stack.count),
                    );
                }
            }
        }

        // 徒手配方
        let craft_y0 = {
            let grid_h = bag_rows(bag_n) as f32 * (BAG_SLOT + BAG_GAP) - BAG_GAP;
            box_r.y + 52.0 + grid_h + 16.0
        };
        label(draw, box_r.x + 20.0, craft_y0, 13.0, ACCENT, "徒手制作");
        for (row, &ri) in hand_recipe_indices().iter().enumerate() {
            let recipe = &RECIPES[ri];
            let r = bag_craft_row_rect(self.screen_w, bag_n, row);
            let can = player.inv.can_pay(recipe.inputs);
            draw.fill_rect(
                r,
                if can {
                    Color::rgb(0.14, 0.28, 0.22)
                } else {
                    Color::rgb(0.18, 0.14, 0.14)
                },
            );
            let need: String = recipe
                .inputs
                .iter()
                .map(|(id, n)| format!("{}×{}", id.label(), n))
                .collect::<Vec<_>>()
                .join("+");
            label(
                draw,
                r.x + 8.0,
                r.y + 7.0,
                12.0,
                TEXT_MAIN,
                &format!("{} ←{}", recipe.label, need),
            );
        }
    }

    pub(crate) fn paint_map(&self, draw: &mut DrawList) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let box_r = map_panel_rect(self.screen_w, self.screen_h);
        panel(draw, box_r, PANEL);
        draw.fill_rect(Rect::new(box_r.x, box_r.y, box_r.w, 2.0), PANEL_EDGE);
        label(
            draw,
            box_r.x + 16.0,
            box_r.y + 12.0,
            18.0,
            TEXT_MAIN,
            "世界地图",
        );
        label(
            draw,
            box_r.x + 120.0,
            box_r.y + 16.0,
            12.0,
            TEXT_DIM,
            "M / Esc 关闭 · 青=出生点 黄=你",
        );

        let mx = box_r.x + 16.0;
        let my = box_r.y + 48.0;
        let mw = box_r.w - 32.0;
        let mh = box_r.h - 72.0;
        draw.fill_rect(Rect::new(mx, my, mw, mh), Color::rgb(0.05, 0.07, 0.10));
        let scale_x = mw / WORLD_W as f32;
        let scale_y = mh / WORLD_H as f32;
        for x in 0..WORLD_W {
            let h = world.surface_at(x);
            let top = my + h as f32 * scale_y;
            let col_h = (my + mh - top).max(2.0);
            let ground = match world.biome_at_x(x) {
                tr_core::BiomeId::Meadow => Color::rgb(0.22, 0.42, 0.22),
                tr_core::BiomeId::Forest => Color::rgb(0.10, 0.32, 0.16),
                tr_core::BiomeId::Desert => Color::rgb(0.55, 0.42, 0.22),
                tr_core::BiomeId::Tundra => Color::rgb(0.42, 0.55, 0.68),
            };
            draw.fill_rect(
                Rect::new(mx + x as f32 * scale_x, top, scale_x.max(1.0), col_h),
                ground,
            );
        }
        let (sx, sy) = world.spawn_pos();
        let stx = wrap_tx((sx / TILE).floor() as i32);
        let sty = (sy / TILE).floor() as i32;
        draw.fill_rect(
            Rect::new(
                mx + stx as f32 * scale_x - 3.0,
                my + sty as f32 * scale_y - 3.0,
                7.0,
                7.0,
            ),
            Color::rgb(0.35, 0.75, 0.95),
        );
        let (px, _, _, _) = player.hitbox();
        let ptx = wrap_tx((px / TILE).floor() as i32);
        let pty = (player.y / TILE).floor() as i32;
        draw.fill_rect(
            Rect::new(
                mx + ptx as f32 * scale_x - 3.0,
                my + pty as f32 * scale_y - 3.0,
                7.0,
                7.0,
            ),
            Color::rgb(1.0, 0.95, 0.55),
        );
    }
}
