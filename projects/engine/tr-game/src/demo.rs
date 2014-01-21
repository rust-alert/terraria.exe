//! 无驱动演示与状态文件。

use tr_core::ItemId;

use crate::enemy::try_melee;
use crate::save::{SessionExtra, save_session};
use crate::world::{TILE, WORLD_H, WORLD_W, wrap_tx};

use crate::app::{Screen, TerrariaApp};

impl TerrariaApp {
    pub(crate) fn tick_demo(&mut self, dt: f32) {
        self.demo_t += dt;
        let t = self.demo_t;
        let phase = self.demo_phase;

        if phase == 0 && t >= 0.7 {
            let base_x = {
                let player = self.player.as_ref().unwrap();
                ((player.x) / TILE).floor() as i32 + 1
            };
            let sh = self.world.as_ref().unwrap().surface_at(base_x);
            let world = self.world.as_mut().unwrap();
            let player = self.player.as_mut().unwrap();
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0)] {
                match player.dig_break(world, base_x + dx, sh + dy) {
                    Some(msg) => tracing::info!(msg, tx = base_x + dx, ty = sh + dy, "demo dig"),
                    None => {}
                }
            }
            // 立刻走近拾取，保证后续放置有材料
            if let Some(msg) = player.pickup_nearby(world) {
                tracing::info!(msg, "demo pickup");
            }
            // 补一点木材方便验证制作
            player.inv.add(ItemId::WOOD, 12);
            player.inv.add(ItemId::SAPLING, 2);
            self.demo_phase = 1;
            tracing::info!(phase = self.demo_phase, "demo phase -> dig done");
            return;
        }

        if phase == 1 && t >= 1.2 {
            let world = self.world.as_mut().unwrap();
            let player = self.player.as_mut().unwrap();
            let tx = ((player.x) / TILE).floor() as i32 + 3;
            let ty = world.surface_at(tx) - 1;
            match player.try_place(world, tx, ty) {
                Some(msg) => tracing::info!(msg, tx, ty, "demo place"),
                None => tracing::warn!(tx, ty, "demo place 无目标"),
            }
            if let Some(msg) = player.try_craft(world, 0) {
                tracing::info!(msg, "demo craft workbench");
            }
            // 先放工作台，再搓工具（避开出生点旁预置火把/木箱）
            player.select_item(ItemId::WORKBENCH);
            let mut wb_placed = false;
            for dx in 4..20 {
                let wb_tx = wrap_tx(tx + dx);
                let wb_ty = world.surface_at(wb_tx) - 1;
                if world.get(wb_tx, wb_ty).solid() {
                    continue;
                }
                // 站在两格外，避免碰撞盒扫到目标格
                player.x = (wb_tx - 2) as f32 * TILE + 2.0;
                player.y = (wb_ty + 1) as f32 * TILE - crate::player::HIT_H;
                if let Some(msg) = player.try_place(world, wb_tx, wb_ty) {
                    if msg == "工作台" {
                        tracing::info!(msg, wb_tx, wb_ty, "demo place workbench");
                        player.x = wb_tx as f32 * TILE + 2.0;
                        wb_placed = true;
                        break;
                    }
                }
            }
            if !wb_placed {
                // 兜底：直接写入世界（demo 不依赖放置重叠判定）
                let wb_tx = wrap_tx(tx + 8);
                let wb_ty = world.surface_at(wb_tx) - 1;
                if player.inv.try_take(ItemId::WORKBENCH, 1) {
                    world.set(wb_tx, wb_ty, tr_core::BlockId::WORKBENCH);
                    player.x = wb_tx as f32 * TILE + 2.0;
                    player.y = (wb_ty + 1) as f32 * TILE - crate::player::HIT_H;
                    tracing::info!(wb_tx, wb_ty, "demo place workbench force");
                    wb_placed = true;
                } else {
                    tracing::warn!("demo place workbench 失败");
                }
            }
            if wb_placed {
                player.inv.add(ItemId::WOOD, 20);
                player.inv.add(ItemId::STONE, 20);
                if let Some(msg) = player.try_craft(world, 2) {
                    tracing::info!(msg, "demo craft wood pick");
                }
                if let Some(msg) = player.try_craft(world, 4) {
                    tracing::info!(msg, "demo craft wood sword");
                }
            }
            // 种植树苗：先贴近目标格再种
            player.select_item(ItemId::SAPLING);
            let mut planted = false;
            for dx in 0..12 {
                let plant_tx = wrap_tx(tx + dx);
                let plant_ty = world.surface_at(plant_tx) - 1;
                if !world.can_plant_sapling(plant_tx, plant_ty) {
                    continue;
                }
                player.x = plant_tx as f32 * TILE + 2.0;
                player.y = (plant_ty + 1) as f32 * TILE - crate::player::HIT_H;
                if let Some(msg) = player.try_place(world, plant_tx, plant_ty) {
                    tracing::info!(msg, plant_tx, plant_ty, "demo plant");
                    planted = true;
                    break;
                }
            }
            if !planted {
                tracing::warn!("demo plant 未找到净空草皮");
            }
            for _ in 0..40 {
                world.tick_growth(0.5);
            }
            // 近战：拉一只怪过来砍
            player.select_item(ItemId::WOOD_SWORD);
            if let Some(e) = self.enemies.first_mut() {
                e.x = player.x + TILE * 1.2;
                e.y = player.y + 10.0;
            }
            if let Some(msg) = try_melee(
                player,
                &mut self.enemies,
                world,
                &mut self.damage_fx,
                &mut self.dust_fx,
            ) {
                tracing::info!(msg, "demo melee");
            }
            // 凝胶食用 + 火把 + 铺墙
            player.inv.add(ItemId::GEL, 3);
            player.select_item(ItemId::GEL);
            player.hp = 40.0;
            if let Some(msg) = player.try_eat() {
                tracing::info!(msg, "demo eat");
            }
            player.inv.add(ItemId::WOOD, 2);
            player.inv.add(ItemId::GEL, 2);
            let torch_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "torch")
                .unwrap_or(5);
            if let Some(msg) = player.try_craft(world, torch_idx) {
                tracing::info!(msg, "demo torch");
            }
            player.select_item(ItemId::TORCH);
            let (px, _, _, _) = player.hitbox();
            let ptx = wrap_tx((px / TILE).floor() as i32) + 2;
            let pty = world.surface_at(ptx) - 2;
            if let Some(msg) = player.try_place(world, ptx, pty) {
                tracing::info!(msg, "demo place torch");
            }
            player.select_item(ItemId::DIRT);
            player.inv.add(ItemId::DIRT, 4);
            if let Some(msg) = player.try_place_wall(world, ptx + 1, pty) {
                tracing::info!(msg, "demo wall");
            }
            // 平台 + 木箱
            player.inv.add(ItemId::WOOD, 20);
            let plat_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "platform")
                .unwrap_or(0);
            if let Some(msg) = player.try_craft(world, plat_idx) {
                tracing::info!(msg, "demo platform craft");
            }
            player.select_item(ItemId::PLATFORM);
            if let Some(msg) = player.try_place(world, ptx, pty - 1) {
                tracing::info!(msg, "demo place platform");
            }
            // 木梯 + 木甲
            let ladder_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "ladder")
                .unwrap_or(0);
            if let Some(msg) = player.try_craft(world, ladder_idx) {
                tracing::info!(msg, "demo ladder craft");
            }
            player.select_item(ItemId::LADDER);
            let ltx = wrap_tx(ptx + 1);
            let lty = world.surface_at(ltx) - 1;
            player.x = (ltx - 2) as f32 * TILE + 2.0;
            if let Some(msg) = player.try_place(world, ltx, lty) {
                tracing::info!(msg, "demo place ladder");
            }
            if let Some(msg) = player.try_place(world, ltx, lty - 1) {
                tracing::info!(msg, "demo place ladder2");
            }
            player.x = ltx as f32 * TILE + 2.0;
            player.y = lty as f32 * TILE;
            let on_l = player.on_ladder(world);
            tracing::info!(on_l, "demo ladder touch");
            player.inv.add(ItemId::WOOD, 20);
            player.inv.add(ItemId::GEL, 8);
            let armor_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "wood_armor")
                .unwrap_or(0);
            // 回工作台搓甲
            if let Some((wbx, wby)) = (0..WORLD_W).find_map(|x| {
                let x = wrap_tx(x);
                (0..WORLD_H)
                    .find(|&y| world.get(x, y) == tr_core::BlockId::WORKBENCH)
                    .map(|y| (x, y))
            }) {
                player.x = wbx as f32 * TILE + 2.0;
                player.y = (wby + 1) as f32 * TILE - crate::player::HIT_H;
            }
            if let Some(msg) = player.try_craft(world, armor_idx) {
                tracing::info!(msg, def = player.defense(), "demo armor");
            }
            // 群系 / 矿物 / 熔炉 / 床
            let bx = wrap_tx(((player.x) / TILE).floor() as i32);
            let biome = world.biome_at_x(bx).name();
            tracing::info!(biome, "demo biome");
            player.inv.add(ItemId::COPPER_ORE, 6);
            player.inv.add(ItemId::IRON_ORE, 3);
            player.inv.add(ItemId::STONE, 20);
            player.inv.add(ItemId::WOOD, 20);
            let furnace_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "furnace")
                .unwrap_or(0);
            if let Some(msg) = player.try_craft(world, furnace_idx) {
                tracing::info!(msg, "demo furnace craft");
            }
            // 强制放熔炉
            if player.inv.get(ItemId::FURNACE) > 0 {
                let ftx = wrap_tx(bx + 4);
                let fty = world.surface_at(ftx) - 1;
                if player.inv.try_take(ItemId::FURNACE, 1) {
                    world.set(ftx, fty, tr_core::BlockId::FURNACE);
                    player.x = ftx as f32 * TILE + 2.0;
                    player.y = (fty + 1) as f32 * TILE - crate::player::HIT_H;
                    tracing::info!(ftx, fty, "demo place furnace");
                }
            }
            let smelt_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "smelt_copper")
                .unwrap_or(0);
            if let Some(msg) = player.try_craft(world, smelt_idx) {
                tracing::info!(msg, "demo smelt copper");
            }
            let bed_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "bed")
                .unwrap_or(0);
            // 床需工作台
            if let Some((wbx, wby)) = (0..WORLD_W).find_map(|x| {
                let x = wrap_tx(x);
                (0..WORLD_H)
                    .find(|&y| world.get(x, y) == tr_core::BlockId::WORKBENCH)
                    .map(|y| (x, y))
            }) {
                player.x = wbx as f32 * TILE + 2.0;
                player.y = (wby + 1) as f32 * TILE - crate::player::HIT_H;
            }
            player.inv.add(ItemId::WOOD, 12);
            player.inv.add(ItemId::GEL, 4);
            if let Some(msg) = player.try_craft(world, bed_idx) {
                tracing::info!(msg, "demo bed craft");
            }
            if player.inv.get(ItemId::BED) > 0 {
                let btx = wrap_tx(((player.x) / TILE).floor() as i32 + 3);
                let bty = world.surface_at(btx) - 1;
                player.select_item(ItemId::BED);
                player.x = (btx - 2) as f32 * TILE + 2.0;
                if let Some(msg) = player.try_place(world, btx, bty) {
                    tracing::info!(msg, home = ?player.home_spawn, "demo place bed");
                } else if player.inv.try_take(ItemId::BED, 1) {
                    world.set(btx, bty, tr_core::BlockId::BED);
                    player.home_spawn = Some((
                        btx as f32 * TILE + TILE * 0.5,
                        bty as f32 * TILE - crate::player::HIT_H,
                    ));
                    tracing::info!(btx, bty, "demo place bed force");
                }
            }
            // 过夜（床）
            let (sx, sy) = world.spawn_pos();
            player.x = sx;
            player.y = sy;
            self.day_t = 150.0;
            player.hp = 50.0;
            self.day_t = 45.0;
            player.refill_vitals();
            tracing::info!(
                day = self.day_t,
                hp = player.hp,
                mp = player.mp,
                "demo sleep skip"
            );
            let chest_idx = crate::craft::RECIPES
                .iter()
                .position(|r| r.id == "chest")
                .unwrap_or(0);
            // 回到工作台旁搓箱
            if let Some((wbx, wby)) = (0..WORLD_W).find_map(|x| {
                let x = wrap_tx(x);
                (0..WORLD_H)
                    .find(|&y| world.get(x, y) == tr_core::BlockId::WORKBENCH)
                    .map(|y| (x, y))
            }) {
                player.x = wbx as f32 * TILE + 2.0;
                player.y = (wby + 1) as f32 * TILE - crate::player::HIT_H;
            }
            if let Some(msg) = player.try_craft(world, chest_idx) {
                tracing::info!(msg, "demo chest craft");
            }
            player.select_item(ItemId::CHEST);
            let mut chest_ok = false;
            for dx in 3..12 {
                let ctx = wrap_tx(((player.x) / TILE).floor() as i32 + dx);
                let cty = world.surface_at(ctx) - 1;
                if world.get(ctx, cty).solid() {
                    continue;
                }
                player.x = (ctx - 2) as f32 * TILE + 2.0;
                player.y = (cty + 1) as f32 * TILE - crate::player::HIT_H;
                if let Some(msg) = player.try_place(world, ctx, cty) {
                    if msg == "木箱" {
                        tracing::info!(msg, ctx, cty, "demo place chest");
                        player.x = ctx as f32 * TILE + 2.0;
                        chest_ok = true;
                        break;
                    }
                }
            }
            if !chest_ok {
                if player.inv.try_take(ItemId::CHEST, 1) {
                    let ctx = wrap_tx(((player.x) / TILE).floor() as i32 + 3);
                    let cty = world.surface_at(ctx) - 1;
                    world.set(ctx, cty, tr_core::BlockId::CHEST);
                    player.x = ctx as f32 * TILE + 2.0;
                    tracing::info!(ctx, cty, "demo place chest force");
                    chest_ok = true;
                } else {
                    tracing::warn!("demo place chest 失败");
                }
            }
            if chest_ok {
                if let Some((cx, cy)) = world.near_chest(
                    player.x,
                    player.y,
                    crate::player::HIT_W,
                    crate::player::HIT_H,
                ) {
                    if player.inv.try_take(ItemId::GEL, 1) {
                        if let Some(chest) = world.chest_at(cx, cy) {
                            *chest.entry(ItemId::GEL).or_insert(0) += 1;
                            tracing::info!(cx, cy, "demo chest deposit");
                        }
                    }
                }
            }
            let (sx, sy) = world.spawn_pos();
            player.x = sx;
            player.y = sy;
            self.survived_night = true;
            let _ = player.inv.add(ItemId::COPPER_ORE, 3);
            match save_session(
                world,
                player,
                &self.enemies,
                SessionExtra {
                    day_t: self.day_t,
                    survived_night: self.survived_night,
                },
            ) {
                Ok(p) => tracing::info!(path = %p.display(), "demo save"),
                Err(e) => tracing::warn!(e, "demo save fail"),
            }
            player.x += 48.0;
            self.demo_phase = 2;
            tracing::info!(phase = self.demo_phase, "demo phase -> place done");
            return;
        }

        if phase == 2 && t >= 1.8 {
            self.demo_phase = 3;
            self.write_status();
            tracing::info!("demo 完成，退出");
            self.exit = true;
        }
    }

    pub(crate) fn write_status(&self) {
        let screen = match self.screen {
            Screen::Title => "title",
            Screen::NewGame => "new_game",
            Screen::Playing => "playing",
            Screen::Pause => "pause",
        };
        let (px, py) = self
            .player
            .as_ref()
            .map(|p| (p.x, p.y))
            .unwrap_or((0.0, 0.0));
        let line = format!(
            "screen={screen} player=({px:.1},{py:.1}) hp={} enemies={} obj={} craft={} demo={}\n",
            self.player.as_ref().map(|p| p.hp).unwrap_or(0.0),
            self.enemies.len(),
            "-",
            self.craft_open,
            self.demo
        );
        let path = std::env::temp_dir().join("terraria_status.txt");
        let _ = std::fs::write(&path, line);
    }
}
