//! 游玩帧更新。

use spark_core::{Rect, Vec2};
use spark_input::{Key, MouseBtn};
use spark_renderer::FrameCtx;
use tr_core::BlockId;

use crate::enemy::{Enemy, update_enemies};
use crate::player::Player;
use crate::save::{SessionExtra, load_session, save_session};
use crate::sfx;
use crate::world::{TILE, WORLD_H, World, wrap_delta_x, wrap_tx, wrap_xf};

use crate::app::{Screen, TerrariaApp};

impl TerrariaApp {
    pub(crate) fn update_playing(&mut self, frame: &FrameCtx<'_>) {
        if self.demo {
            self.tick_demo(frame.dt);
        }

        if frame.input.key_pressed(Key::Escape) {
            if self.shop_open {
                self.shop_open = false;
            } else if self.chest_open.is_some() {
                self.chest_open = None;
                self.flush_cursor_to_inv();
            } else if self.map_open {
                self.map_open = false;
            } else if self.craft_open {
                self.craft_open = false;
            } else if self.bag_open {
                self.bag_open = false;
                self.flush_cursor_to_inv();
            } else if self.cursor_stack.is_some() {
                self.flush_cursor_to_inv();
            } else {
                self.screen = Screen::Pause;
            }
            return;
        }

        if frame.input.key_pressed(Key::C) {
            self.craft_open = !self.craft_open;
            if self.craft_open {
                self.bag_open = false;
                self.map_open = false;
                self.shop_open = false;
                self.flush_cursor_to_inv();
            }
        }
        if frame.input.key_pressed(Key::I) {
            self.toggle_bag();
        }
        if frame.input.key_pressed(Key::H) {
            if let (Some(world), Some(player)) = (self.world.as_ref(), self.player.as_ref()) {
                let (px, py, pw, ph) = player.hitbox();
                let (msg, tiles) = crate::housing::query_near(world, px + pw * 0.5, py + ph * 0.5);
                self.house_query_ok = msg.contains("合格");
                self.house_query_tiles = tiles;
                self.house_flash_t = 2.5;
                self.set_toast(msg);
            }
        }
        if frame.input.key_pressed(Key::M) {
            self.toggle_map();
        }
        if frame.input.key_pressed(Key::F3) {
            self.debug_hud = !self.debug_hud;
        }

        // 快捷栏 1-9 / 0；制作面板改点选，不再抢数字键。
        if !self.bag_open && !self.shop_open {
            for (i, key) in [
                Key::Digit1,
                Key::Digit2,
                Key::Digit3,
                Key::Digit4,
                Key::Digit5,
                Key::Digit6,
                Key::Digit7,
                Key::Digit8,
                Key::Digit9,
                Key::Digit0,
            ]
            .iter()
            .enumerate()
            {
                if frame.input.key_pressed(*key) {
                    if let Some(p) = self.player.as_mut() {
                        p.select_hotbar(i);
                    }
                }
            }
        }

        // 滚轮切快捷栏（制作/传送/箱子打开时不切，避免误触）。
        if !self.craft_open && !self.bag_open && !self.shop_open && self.chest_open.is_none() {
            let wheel = frame.input.wheel();
            if wheel.abs() > 0.01 {
                if let Some(p) = self.player.as_mut() {
                    let len = crate::player::HOTBAR_LEN;
                    let cur = p.inv.hotbar_sel % len;
                    let next = if wheel > 0.0 {
                        cur.checked_sub(1).unwrap_or(len - 1)
                    } else {
                        (cur + 1) % len
                    };
                    p.select_hotbar(next);
                }
            }
        }

        // 开发会话快照（非正版 .wld）
        if frame.input.key_pressed(Key::F5) {
            if let (Some(world), Some(player)) = (self.world.as_ref(), self.player.as_ref()) {
                let extra = SessionExtra {
                    day_t: self.day_t,
                    survived_night: self.survived_night,
                };
                match save_session(world, player, &self.enemies, extra) {
                    Ok(p) => self.set_toast(format!("已写入开发会话快照 {}", p.display())),
                    Err(e) => self.set_toast(format!("会话快照失败：{e}")),
                }
            }
        }
        if frame.input.key_pressed(Key::F9) {
            if let (Some(world), Some(player)) = (self.world.as_mut(), self.player.as_mut()) {
                let mut extra = SessionExtra {
                    day_t: self.day_t,
                    survived_night: self.survived_night,
                };
                match load_session(world, player, &mut self.enemies, &mut extra) {
                    Ok(()) => {
                        self.day_t = extra.day_t;
                        self.survived_night = extra.survived_night;
                        self.set_toast("已读取开发会话快照");
                    }
                    Err(e) => self.set_toast(format!("读取会话快照失败：{e}")),
                }
            }
        }

        let mut toast: Option<String> = None;
        let (mx, my) = frame.input.mouse_pos();
        let shift = frame.input.key_down(Key::LShift) || frame.input.key_down(Key::RShift);
        let mut hud_consumed = false;
        if frame.input.mouse_pressed(MouseBtn::Left) {
            let (hit, msg) = self.handle_hud_button(mx, my, true, shift);
            hud_consumed = hit;
            if let Some(msg) = msg {
                toast = Some(msg);
            }
        } else if frame.input.mouse_pressed(MouseBtn::Right)
            && (self.bag_open || self.chest_open.is_some())
        {
            let (hit, msg) = self.handle_hud_button(mx, my, false, false);
            hud_consumed = hit;
            if let Some(msg) = msg {
                toast = Some(msg);
            }
        }
        let over_hud = hud_consumed || self.hud_pointer_at(mx, my) != crate::hud::HudPointer::None;

        {
            let Some(world) = self.world.as_mut() else {
                return;
            };
            let Some(player) = self.player.as_mut() else {
                return;
            };

            player.update(world, frame.input, frame.dt);
            if self.house_flash_t > 0.0 {
                self.house_flash_t = (self.house_flash_t - frame.dt).max(0.0);
            }
            if player.just_landed {
                let impact = 0.4 + (player.vx.abs() * 0.002).min(0.45);
                crate::fx::burst_dust(
                    &mut self.dust_fx,
                    player.x + crate::player::HIT_W * 0.5,
                    player.y + crate::player::HIT_H,
                    impact,
                );
            }
            self.day_t = (self.day_t + frame.dt) % 180.0;
            let night = night_factor(self.day_t);
            if night > 0.55 && !self.night_warned {
                self.night_warned = true;
                toast = Some("夜幕降临……点亮火把，靠近床按 E 可过夜。".into());
            }
            if night < 0.2 {
                self.night_warned = false;
            }
            world.tick_drops(frame.dt);
            world.tick_growth(frame.dt);
            // 流体步进已冻结：当前为未经验证的水位模型，待原版 0..=255 液量替换。
            let night_cap = if night > 0.85 { 16 } else { 12 };
            if night > 0.55 && self.enemies.len() < night_cap {
                maybe_spawn_night_enemy(world, &mut self.enemies, player, night);
            }
            update_enemies(
                world,
                &mut self.enemies,
                player,
                frame.dt,
                night,
                &self.sfx,
                &self.audio,
            );
            crate::fx::tick_floaters(&mut self.damage_fx, frame.dt);
            crate::fx::tick_dust(&mut self.dust_fx, frame.dt);
            if let Some(msg) = crate::weapon::update_projectiles(
                world,
                &mut self.projectiles,
                &mut self.enemies,
                &mut self.damage_fx,
                &mut self.dust_fx,
                frame.dt,
            ) {
                toast = Some(msg);
            }
            if player.pending_respawn {
                let (sx, sy) = player.home_spawn.unwrap_or_else(|| world.spawn_pos());
                player.respawn_at(sx, sy);
                toast = Some(if player.home_spawn.is_some() {
                    "你在床边醒来……".into()
                } else {
                    "你在出生点醒来……".into()
                });
            }
            if let Some(msg) = player.pickup_nearby(world) {
                if msg.contains("铜币") {
                    sfx::coins(&self.sfx, &self.audio);
                } else {
                    sfx::pickup(&self.sfx, &self.audio);
                }
                toast = Some(msg);
            }

            let mut flush_cursor = false;
            let e_pressed = frame.input.key_pressed(Key::E);
            let (px, py, pw, ph) = player.hitbox();
            let chest_here = world.near_chest(px, py, pw, ph);
            let bed_here = world.near_bed(px, py, pw, ph);
            let station_here =
                world.near_workbench(px, py, pw, ph) || world.near_furnace(px, py, pw, ph);
            let guide_i = self
                .town_npcs
                .iter()
                .position(|n| n.near_player(px, py, pw, ph));
            if e_pressed {
                if let Some(i) = guide_i {
                    let kind = self.town_npcs[i].kind;
                    if kind == crate::npc::TownKind::Merchant {
                        self.shop_open = !self.shop_open;
                        if self.shop_open {
                            self.craft_open = false;
                            self.bag_open = false;
                            self.map_open = false;
                            self.chest_open = None;
                            if let Some(left) = crate::container::absorb_cursor(
                                &mut player.inv,
                                &mut self.cursor_stack,
                            ) {
                                let tx = wrap_tx(
                                    ((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32,
                                );
                                let ty =
                                    ((player.y + crate::player::HIT_H * 0.5) / TILE).floor() as i32;
                                world.spawn_drop_at_tile(tx, ty, left.id, left.count);
                            }
                        }
                        sfx::chat(&self.sfx, &self.audio);
                        toast = Some(if self.shop_open {
                            "商人商店 · 点击购买 · Esc 关闭".into()
                        } else {
                            "已关闭商店".into()
                        });
                    } else {
                        let line = self.town_npcs[i].talk_next();
                        let name = kind.label();
                        sfx::chat(&self.sfx, &self.audio);
                        toast = Some(format!("{name}：{line}"));
                    }
                } else if let Some(pos) = chest_here {
                    if self.chest_open == Some(pos) {
                        self.chest_open = None;
                        flush_cursor = true;
                    } else {
                        self.chest_open = Some(pos);
                        self.craft_open = false;
                        self.bag_open = true;
                        self.map_open = false;
                        self.shop_open = false;
                        toast = Some("木箱 · 拖放物品，Shift 快移".into());
                    }
                } else if bed_here.is_some() {
                    let night = night_factor(self.day_t);
                    let (msg, slept) = try_rest(night, true, &mut self.day_t, player, world);
                    if slept {
                        self.survived_night = true;
                    }
                    toast = Some(msg);
                } else if station_here {
                    self.craft_open = !self.craft_open;
                    if self.craft_open {
                        self.bag_open = false;
                        self.chest_open = None;
                        self.shop_open = false;
                        toast = Some("制作台已打开".into());
                    } else {
                        toast = Some("已关闭制作".into());
                    }
                } else {
                    toast = Some("附近没有可交互的设施".into());
                }
            }

            if let Some((cx, cy)) = self.chest_open {
                if world.get(cx, cy) != BlockId::CHEST {
                    self.chest_open = None;
                    flush_cursor = true;
                }
            }

            if flush_cursor {
                if let Some(left) =
                    crate::container::absorb_cursor(&mut player.inv, &mut self.cursor_stack)
                {
                    let tx =
                        wrap_tx(((player.x + crate::player::HIT_W * 0.5) / TILE).floor() as i32);
                    let ty = ((player.y + crate::player::HIT_H * 0.5) / TILE).floor() as i32;
                    world.spawn_drop_at_tile(tx, ty, left.id, left.count);
                }
            }

            if frame.input.key_pressed(Key::Q) {
                if player.grapple.is_some() {
                    player.grapple = None;
                    toast = Some("收回钩爪".into());
                } else if player.selected_item().is_grapple() && player.can_swing() {
                    let aim_x = mx + self.cam_x;
                    let aim_y = my + self.cam_y;
                    if let Some(msg) = crate::grapple::try_use(player, aim_x, aim_y) {
                        if msg.contains("抛出") {
                            sfx::place(&self.sfx, &self.audio);
                        }
                        toast = Some(msg);
                    }
                }
            }

            let tx = ((mx + self.cam_x) / TILE).floor() as i32;
            let ty = ((my + self.cam_y) / TILE).floor() as i32;

            if !over_hud
                && !self.craft_open
                && !self.bag_open
                && !self.shop_open
                && self.chest_open.is_none()
            {
                let pressed = frame.input.mouse_pressed(MouseBtn::Left);
                let held = frame.input.mouse_down(MouseBtn::Left);
                if pressed || held {
                    let aim_x = mx + self.cam_x;
                    let aim_y = my + self.cam_y;
                    if let Some(out) = crate::use_item::primary_use(
                        player,
                        world,
                        &mut self.enemies,
                        &mut self.damage_fx,
                        &mut self.dust_fx,
                        &mut self.projectiles,
                        tx,
                        ty,
                        aim_x,
                        aim_y,
                        pressed,
                        held,
                    ) {
                        apply_primary(&self.sfx, &self.audio, &mut self.dust_fx, &mut toast, out);
                    }
                }
                if frame.input.mouse_pressed(MouseBtn::Right) {
                    if let Some(msg) = crate::use_item::secondary_use(player, world, tx, ty) {
                        if msg.ends_with("墙") {
                            sfx::place(&self.sfx, &self.audio);
                        }
                        toast = Some(msg);
                    }
                }
            }

            let (px, py, pw, ph) = player.hitbox();
            let target_x = px + pw * 0.5 - frame.screen_w * 0.5;
            let target_y = py + ph * 0.5 - frame.screen_h * 0.5;
            let dx = wrap_delta_x(self.cam_x, target_x);
            self.cam_x = wrap_xf(self.cam_x + dx * (1.0 - (-8.0 * frame.dt).exp()));
            self.cam_y += (target_y - self.cam_y) * (1.0 - (-8.0 * frame.dt).exp());
            let max_y = WORLD_H as f32 * TILE - frame.screen_h;
            self.cam_y = self.cam_y.clamp(0.0, max_y.max(0.0));
        }

        if let Some(msg) = toast {
            self.set_toast(msg);
        }
        if self.toast_t > 0.0 {
            self.toast_t -= frame.dt;
        }

        self.status_acc += frame.dt;
        if self.status_acc >= 0.5 {
            self.status_acc = 0.0;
            self.write_status();
        }
    }
}

fn try_rest(
    night: f32,
    near_bed: bool,
    day_t: &mut f32,
    player: &mut Player,
    world: &World,
) -> (String, bool) {
    if night <= 0.35 {
        return ("白日无法入睡，入夜再按 E 休息。".into(), false);
    }
    *day_t = 45.0;
    player.refill_vitals();
    player.iframes = 1.0;
    if near_bed {
        let (px, py, pw, ph) = player.hitbox();
        if let Some((bx, by)) = world.near_bed(px, py, pw, ph) {
            let hx = bx as f32 * TILE + TILE * 0.5;
            let hy = by as f32 * TILE - crate::player::HIT_H;
            player.home_spawn = Some((hx, hy));
        }
        ("在床上睡过一夜……家园已确认。".into(), true)
    } else {
        ("睡过一夜……天已破晓。".into(), true)
    }
}

/// 0 白昼，1 深夜。周期 180s，约后 70s 入夜。
fn night_factor(day_t: f32) -> f32 {
    let phase = day_t / 180.0;
    if phase < 0.25 {
        1.0 - phase / 0.25
    } else if phase < 0.55 {
        0.0
    } else if phase < 0.75 {
        (phase - 0.55) / 0.2
    } else {
        1.0
    }
}

fn apply_primary(
    bank: &crate::sfx::SfxBank,
    audio: &spark_audio::AudioBus,
    dust: &mut Vec<crate::fx::DustParticle>,
    toast: &mut Option<String>,
    out: crate::use_item::PrimaryOutcome,
) {
    use crate::use_item::PrimaryOutcome;
    match out {
        PrimaryOutcome::Eat(msg) => {
            if msg.starts_with("食用") {
                sfx::pickup(bank, audio);
            }
            *toast = Some(msg);
        }
        PrimaryOutcome::Place { msg, tx, ty } => {
            if msg != "太远了"
                && msg != "占着自己"
                && !msg.starts_with("没有")
                && !msg.starts_with("需")
                && msg != "不可放置"
            {
                sfx::place(bank, audio);
                crate::fx::burst_dust(
                    dust,
                    tx as f32 * TILE + TILE * 0.5,
                    ty as f32 * TILE + TILE,
                    0.4,
                );
            }
            *toast = Some(format!("放置 {msg}"));
        }
        PrimaryOutcome::Dig { msg, tx, ty } => {
            cue_dig_fx(bank, audio, dust, tx, ty, &msg);
            *toast = Some(msg);
        }
        PrimaryOutcome::Melee(msg) => {
            sfx::melee_hit(bank, audio);
            *toast = Some(msg);
        }
        PrimaryOutcome::Ranged(msg) => {
            if msg.contains('−') || msg.contains("远程") || msg.contains("魔力") {
                sfx::melee_hit(bank, audio);
            }
            *toast = Some(msg);
        }
    }
}

fn cue_dig(bank: &crate::sfx::SfxBank, audio: &spark_audio::AudioBus, msg: &str) {
    if msg == "太远了" {
        return;
    }
    if msg.contains("掉落") || msg.contains("碎了") {
        sfx::dig_break(bank, audio);
    } else if msg.contains("石") || msg.contains("矿") {
        sfx::dig_tink(bank, audio);
    } else {
        sfx::dig_chip(bank, audio);
    }
}

fn cue_dig_fx(
    bank: &crate::sfx::SfxBank,
    audio: &spark_audio::AudioBus,
    dust: &mut Vec<crate::fx::DustParticle>,
    tx: i32,
    ty: i32,
    msg: &str,
) {
    cue_dig(bank, audio, msg);
    if msg == "太远了" {
        return;
    }
    let x = tx as f32 * TILE + TILE * 0.5;
    let y = ty as f32 * TILE + TILE * 0.5;
    if msg.contains("掉落") || msg.contains("碎了") {
        crate::fx::burst_chips(dust, x, y);
        crate::fx::burst_dust(dust, x, y + TILE * 0.25, 0.7);
    } else {
        crate::fx::burst_dust(dust, x, y, 0.35);
    }
}

fn maybe_spawn_night_enemy(world: &World, enemies: &mut Vec<Enemy>, player: &Player, night: f32) {
    let (px, _, _, _) = player.hitbox();
    let side = if ((px * 0.01) as i32).rem_euclid(2) == 0 {
        10
    } else {
        -10
    };
    let dist = 8 + ((night * 6.0) as i32);
    let tx = wrap_tx(((px / TILE).floor() as i32) + side * dist / 10);
    let already = enemies
        .iter()
        .any(|e| wrap_delta_x(e.x, tx as f32 * TILE).abs() < TILE * 3.0);
    if already {
        return;
    }
    let sh = world.surface_at(tx);
    let roll = (world.seed ^ (tx as u64).wrapping_mul(19)) % 5;
    // 入夜后空中刷恶魔眼，深夜地面刷僵尸，其余史莱姆。
    if night > 0.58 && roll < 2 {
        let air = 4 + (roll as i32);
        enemies.push(Enemy::demon_eye(
            tx as f32 * TILE + 2.0,
            sh as f32 * TILE - TILE * air as f32,
        ));
    } else if night > 0.72 {
        enemies.push(Enemy::zombie(
            tx as f32 * TILE + 2.0,
            sh as f32 * TILE - TILE * (42.0 / 16.0),
        ));
    } else {
        enemies.push(Enemy::slime(
            tx as f32 * TILE + 2.0,
            sh as f32 * TILE - TILE * 0.7,
        ));
    }
}
