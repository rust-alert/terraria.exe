//! 玩家移动控制器：加速/制动、可变跳高、单向平台下穿。
//!
//! 土狼时间与跳跃缓冲已移除：在正版测量基线建立前，只保留落地起跳与松键削峰。

use spark_input::{Input, Key};

use crate::grapple;
use crate::world::{TILE, World};

use super::{HIT_H, HIT_W, Player};

/// 最大水平速度（像素/秒）。
pub(crate) const MOVE_SPEED: f32 = TILE * 12.0;
pub(crate) const JUMP_V: f32 = -TILE * 18.0;
pub(crate) const GRAVITY: f32 = TILE * 48.0;

const GROUND_ACCEL: f32 = MOVE_SPEED * 14.0;
const GROUND_BRAKE: f32 = MOVE_SPEED * 18.0;
const AIR_ACCEL: f32 = MOVE_SPEED * 7.0;
const AIR_BRAKE: f32 = MOVE_SPEED * 4.0;
/// 松开跳跃键后保留的上升速度比例。
const JUMP_CUT: f32 = 0.42;
const DROP_THROUGH_TIME: f32 = 0.22;

/// 本帧移动意图（由按键采样）。
struct MoveIntent {
    ax: f32,
    jump_pressed: bool,
    jump_down: bool,
    climb_y: f32,
    want_drop_through: bool,
}

impl MoveIntent {
    fn sample(input: &Input) -> Self {
        let mut ax = 0.0;
        if input.key_down(Key::A) || input.key_down(Key::Left) {
            ax -= 1.0;
        }
        if input.key_down(Key::D) || input.key_down(Key::Right) {
            ax += 1.0;
        }
        let jump_pressed = input.key_pressed(Key::Space);
        let jump_down = input.key_down(Key::Space);
        let mut climb_y = 0.0;
        if input.key_down(Key::W) || input.key_down(Key::Up) {
            climb_y -= 1.0;
        }
        if input.key_down(Key::S) || input.key_down(Key::Down) {
            climb_y += 1.0;
        }
        // S + Space：从单向平台下穿（与跳跃同键，故下穿优先于起跳）。
        let want_drop_through =
            (input.key_down(Key::S) || input.key_down(Key::Down)) && jump_pressed;
        Self {
            ax,
            jump_pressed,
            jump_down,
            climb_y,
            want_drop_through,
        }
    }
}

/// 推进移动、攀爬、游泳与落地伤害相关状态。钩爪牵引仍在本模块末尾衔接。
pub(crate) fn tick(player: &mut Player, world: &World, input: &Input, dt: f32) {
    let intent = MoveIntent::sample(input);
    if intent.ax != 0.0 {
        player.facing = intent.ax.signum();
    }

    player.drop_through_t = (player.drop_through_t - dt).max(0.0);

    let climbing = player.on_ladder(world);
    let wet = world.water_overlap_ratio(player.x, player.y, HIT_W, HIT_H);
    let swimming = wet > 0.35 && !climbing;

    if climbing {
        apply_horizontal(player, intent.ax, true, dt);
        player.vy = intent.climb_y * MOVE_SPEED * 0.85;
        player.fall_start_y = None;
        if intent.jump_pressed && !intent.want_drop_through {
            player.vy = JUMP_V * 0.85;
        }
    } else if swimming {
        apply_horizontal(player, intent.ax, false, dt);
        player.vx = player.vx.clamp(-MOVE_SPEED * 0.55, MOVE_SPEED * 0.55);
        let mut swim = intent.climb_y;
        if intent.jump_down {
            swim -= 1.0;
        }
        swim = swim.clamp(-1.0, 1.0);
        player.vy += GRAVITY * dt * 0.18;
        player.vy += swim * MOVE_SPEED * 0.7 * dt * 8.0;
        player.vy *= 0.92;
        player.vy = player.vy.clamp(-MOVE_SPEED * 0.9, MOVE_SPEED * 0.7);
        player.fall_start_y = None;
    } else {
        apply_horizontal(player, intent.ax, player.on_ground, dt);

        if player.on_ground {
            player.extra_jumps = if player.inv.has_cloud_jump() { 1 } else { 0 };
        }

        if intent.want_drop_through && player.on_ground {
            // 脚底所在格若为平台（或紧贴平台顶）则开启下穿窗。
            if standing_on_platform(player, world) {
                player.drop_through_t = DROP_THROUGH_TIME;
                player.on_ground = false;
                player.vy = MOVE_SPEED * 0.35;
                player.fall_start_y = Some(player.y + HIT_H);
            }
        }

        if intent.jump_pressed && player.on_ground && player.drop_through_t <= 0.0 {
            player.vy = JUMP_V;
            player.on_ground = false;
            player.fall_start_y = Some(player.y + HIT_H);
            player.jump_cut_armed = true;
        } else if intent.jump_pressed
            && !player.on_ground
            && player.extra_jumps > 0
            && player.inv.has_cloud_jump()
            && player.grapple.is_none()
            && player.drop_through_t <= 0.0
        {
            player.vy = JUMP_V * 0.92;
            player.extra_jumps -= 1;
            player.fall_start_y = Some(player.y + HIT_H);
            player.jump_cut_armed = true;
        }

        // 可变跳高：上升段松开 Space 则削减上升速度。
        if player.jump_cut_armed && !intent.jump_down && player.vy < 0.0 {
            player.vy *= JUMP_CUT;
            player.jump_cut_armed = false;
        }
        if player.on_ground || player.vy >= 0.0 {
            player.jump_cut_armed = false;
        }

        player.vy += GRAVITY * dt;
    }

    // 钩爪改由 Q 解除（见 play）；此处不再用 Space 扯断，以免与跳跃冲突。
    grapple::apply_pull(player);

    let was_grounded = player.on_ground;
    player.just_landed = false;
    player.move_axis(world, player.vx * dt, 0.0);
    player.move_axis(world, 0.0, player.vy * dt);
    grapple::advance_hook(player, world, dt);

    if !climbing && !swimming && was_grounded && !player.on_ground && player.fall_start_y.is_none()
    {
        player.fall_start_y = Some(player.y + HIT_H);
    }
    if !climbing && !swimming && player.on_ground {
        if !was_grounded {
            player.just_landed = true;
        }
        if let Some(start) = player.fall_start_y.take() {
            let dist = (player.y + HIT_H - start).max(0.0);
            let safe = TILE * 12.0;
            if dist > safe && player.iframes <= 0.0 {
                let dmg = ((dist - safe) / TILE * 6.0).min(40.0);
                if dmg >= 1.0 {
                    player.hurt(dmg);
                }
            }
        }
    }
}

fn apply_horizontal(player: &mut Player, ax: f32, grounded: bool, dt: f32) {
    let max_speed = MOVE_SPEED;
    let (accel, brake) = if grounded {
        (GROUND_ACCEL, GROUND_BRAKE)
    } else {
        (AIR_ACCEL, AIR_BRAKE)
    };
    if ax != 0.0 {
        let target = ax * max_speed;
        // 反向时用制动加速转向。
        let turning = player.vx != 0.0 && player.vx.signum() != ax.signum();
        let rate = if turning { brake + accel } else { accel };
        player.vx = approach(player.vx, target, rate * dt);
    } else {
        player.vx = approach(player.vx, 0.0, brake * dt);
    }
    player.vx = player.vx.clamp(-max_speed, max_speed);
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    let delta = target - current;
    if delta.abs() <= max_delta {
        target
    } else {
        current + delta.signum() * max_delta
    }
}

fn standing_on_platform(player: &Player, world: &World) -> bool {
    let foot_y = player.y + HIT_H + 0.5;
    let ty = (foot_y / TILE).floor() as i32;
    let cx = player.x + HIT_W * 0.5;
    let tx = (cx / TILE).floor() as i32;
    for dx in -1..=1 {
        if world.get(tx + dx, ty).is_platform() {
            return true;
        }
    }
    false
}
