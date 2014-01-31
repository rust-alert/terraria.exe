//! 固定步长移动回放。
//!
//! 记录每 tick 的输入、位置和速度。这只是可重复的记录格式。
//! 数值尚未对照正版样本，不能当成手感验收。

use spark_input::{ButtonState, Input, Key};

use crate::world::{SPAWN_TX, TILE, World};

use super::{HIT_H, Player};

/// 模拟步长（秒）。渲染插值不得反向改这些样本。
pub const SIM_DT: f32 = 1.0 / 60.0;

/// 一 tick 的移动按键。`jump` 为按住，上升沿才触发起跳。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlTick {
    /// 按住右。
    pub right: bool,
    /// 按住左。
    pub left: bool,
    /// 按住跳跃。
    pub jump: bool,
}

/// 一 tick 结束后的运动样本。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSample {
    /// 从 0 起的 tick 序号。
    pub tick: u32,
    /// 碰撞盒左缘。
    pub x: f32,
    /// 碰撞盒上缘。
    pub y: f32,
    /// 水平速度。
    pub vx: f32,
    /// 竖直速度。向下为正。
    pub vy: f32,
    /// 本 tick 结束后是否着地。
    pub on_ground: bool,
}

/// 按控制序列推进玩家，返回逐步样本。
pub fn replay(world: &World, player: &mut Player, controls: &[ControlTick]) -> Vec<MotionSample> {
    let mut input = Input::default();
    let mut prev = ControlTick::default();
    let mut out = Vec::with_capacity(controls.len());
    for (i, now) in controls.iter().copied().enumerate() {
        input.begin_frame();
        sync_key(&mut input, Key::D, prev.right, now.right);
        sync_key(&mut input, Key::A, prev.left, now.left);
        sync_key(&mut input, Key::Space, prev.jump, now.jump);
        player.update(world, &input, SIM_DT);
        out.push(MotionSample {
            tick: i as u32,
            x: player.x,
            y: player.y,
            vx: player.vx,
            vy: player.vy,
            on_ground: player.on_ground,
        });
        prev = now;
    }
    out
}

fn sync_key(input: &mut Input, key: Key, was: bool, now: bool) {
    if now && !was {
        input.on_key(key, ButtonState::Pressed);
    } else if !now && was {
        input.on_key(key, ButtonState::Released);
    }
}

/// 把玩家放到出生列地表上，供回放起步。
pub fn place_on_spawn(world: &World) -> Player {
    let tx = SPAWN_TX;
    let surface = world.surface_at(tx);
    let x = tx as f32 * TILE;
    let y = surface as f32 * TILE - HIT_H;
    let mut player = Player::new(x, y);
    player.on_ground = true;
    player
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hold_right(n: usize) -> Vec<ControlTick> {
        vec![
            ControlTick {
                right: true,
                ..ControlTick::default()
            };
            n
        ]
    }

    #[test]
    fn run_samples_accelerate_then_brake() {
        let world = World::generate(1);
        let mut player = place_on_spawn(&world);
        let mut controls = hold_right(20);
        controls.extend(std::iter::repeat_n(ControlTick::default(), 20));
        let samples = replay(&world, &mut player, &controls);
        assert_eq!(samples.len(), 40);
        assert!(samples[10].vx > samples[0].vx);
        assert!(samples[19].vx > 1.0);
        assert!(samples[39].vx.abs() < samples[19].vx.abs());
    }

    #[test]
    fn held_jump_rises_higher_than_tap() {
        let world = World::generate(1);
        let mut tap_player = place_on_spawn(&world);
        let mut hold_player = place_on_spawn(&world);
        let mut tap = vec![ControlTick {
            jump: true,
            ..ControlTick::default()
        }];
        tap.extend(std::iter::repeat_n(ControlTick::default(), 40));
        let hold = vec![
            ControlTick {
                jump: true,
                ..ControlTick::default()
            };
            41
        ];
        let tap_s = replay(&world, &mut tap_player, &tap);
        let hold_s = replay(&world, &mut hold_player, &hold);
        let tap_min_y = tap_s.iter().map(|s| s.y).fold(f32::MAX, f32::min);
        let hold_min_y = hold_s.iter().map(|s| s.y).fold(f32::MAX, f32::min);
        assert!(hold_min_y < tap_min_y);
    }

    #[test]
    fn fall_from_air_gains_downward_speed() {
        let world = World::generate(1);
        let mut player = place_on_spawn(&world);
        player.y -= TILE * 8.0;
        player.on_ground = false;
        let samples = replay(&world, &mut player, &[ControlTick::default(); 15]);
        assert!(samples[14].vy > samples[0].vy);
        assert!(samples[14].y > samples[0].y);
    }
}
