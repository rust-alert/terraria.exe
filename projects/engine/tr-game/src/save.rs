//! 会话快档：方块全表 + 玩家 / 目标 / 日时（无第三方序列化）。

use std::path::PathBuf;

use tr_core::ItemId;

use crate::enemy::Enemy;
use crate::objective::Objective;
use crate::player::Player;
use crate::world::World;

const MAGIC_V1: &str = "ASTRACRAFT_SAVE_V1";
const MAGIC_V2: &str = "ASTRACRAFT_SAVE_V2";
const MAGIC_V3: &str = "ASTRACRAFT_SAVE_V3";

/// 与方块/背包并列的会话元数据。
#[derive(Debug, Clone, Copy)]
pub struct SessionExtra {
    pub day_t: f32,
    pub seen_warp: bool,
    pub warped_once: bool,
    pub survived_night: bool,
    pub objective: Objective,
}

impl Default for SessionExtra {
    fn default() -> Self {
        Self {
            day_t: 40.0,
            seen_warp: false,
            warped_once: false,
            survived_night: false,
            objective: Objective::GatherWood,
        }
    }
}

pub fn quick_save_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("terraria_quick.sav")
}

fn objective_tag(o: Objective) -> &'static str {
    match o {
        Objective::GatherWood => "gather_wood",
        Objective::CraftWorkbench => "craft_workbench",
        Objective::PlaceWorkbench => "place_workbench",
        Objective::CraftPick => "craft_pick",
        Objective::PlaceBed => "place_bed",
        Objective::SurviveNight => "survive_night",
        Objective::CraftStonePick => "craft_stone_pick",
        Objective::MineCopper => "mine_copper",
        Objective::FindWarp => "find_warp",
        Objective::WarpOnce => "warp_once",
        Objective::Done => "done",
    }
}

fn parse_objective(s: &str) -> Option<Objective> {
    Some(match s {
        "gather_wood" => Objective::GatherWood,
        "craft_workbench" => Objective::CraftWorkbench,
        "place_workbench" => Objective::PlaceWorkbench,
        "craft_pick" => Objective::CraftPick,
        "place_bed" => Objective::PlaceBed,
        "survive_night" => Objective::SurviveNight,
        "craft_stone_pick" => Objective::CraftStonePick,
        "mine_copper" => Objective::MineCopper,
        "find_warp" => Objective::FindWarp,
        "warp_once" => Objective::WarpOnce,
        "done" => Objective::Done,
        _ => return None,
    })
}

pub fn save_session(
    world: &World,
    player: &Player,
    enemies: &[Enemy],
    extra: SessionExtra,
) -> Result<PathBuf, String> {
    let path = quick_save_path();
    let mut out = String::new();
    out.push_str(MAGIC_V3);
    out.push('\n');
    out.push_str(&format!("seed={}\n", world.seed));
    out.push_str(&format!(
        "meta={:.3},{},{},{},{}\n",
        extra.day_t,
        u8::from(extra.seen_warp),
        u8::from(extra.warped_once),
        objective_tag(extra.objective),
        u8::from(extra.survived_night),
    ));
    out.push_str(&format!(
        "player={:.3},{:.3},{:.1},{:.1},{},{:.1},{:.1},{:.1},{:.1}\n",
        player.x,
        player.y,
        player.hp,
        player.max_hp,
        player.inv.hotbar_sel,
        player.facing,
        player.vx,
        player.mp,
        player.max_mp
    ));
    if let Some((hx, hy)) = player.home_spawn {
        out.push_str(&format!("home={hx:.3},{hy:.3}\n"));
    } else {
        out.push_str("home=\n");
    }
    out.push_str("hotbar=");
    out.push_str(&player.inv.encode_hotbar());
    out.push('\n');
    out.push_str("bag=");
    out.push_str(&player.inv.encode_bag());
    out.push('\n');
    out.push_str("armor=");
    out.push_str(&player.inv.encode_armor());
    out.push('\n');
    out.push_str("accessory=");
    out.push_str(&player.inv.encode_accessory());
    out.push('\n');
    // 兼容旧读档：仍写汇总 inv 行
    let inv = player
        .inv
        .entries()
        .into_iter()
        .map(|(id, n)| format!("{}:{}", id.0, n))
        .collect::<Vec<_>>()
        .join(",");
    out.push_str("inv=");
    out.push_str(&inv);
    out.push('\n');
    out.push_str("blocks=");
    out.push_str(&world.encode_blocks());
    out.push('\n');
    out.push_str("walls=");
    out.push_str(&world.encode_walls());
    out.push('\n');
    out.push_str("chests=");
    out.push_str(&world.encode_chests());
    out.push('\n');
    out.push_str("fluids=");
    out.push_str(&world.encode_fluids());
    out.push('\n');
    out.push_str("tdur=\n");
    out.push_str("enemies=");
    let eline = enemies
        .iter()
        .map(|e| format!("{:.2},{:.2},{:.1},{:.1}", e.x, e.y, e.hp, e.facing))
        .collect::<Vec<_>>()
        .join(";");
    out.push_str(&eline);
    out.push('\n');
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn load_session(
    world: &mut World,
    player: &mut Player,
    enemies: &mut Vec<Enemy>,
    extra: &mut SessionExtra,
) -> Result<(), String> {
    let path = quick_save_path();
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut lines = text.lines();
    let magic = lines.next().ok_or("空存档")?;
    if magic != MAGIC_V1 && magic != MAGIC_V2 && magic != MAGIC_V3 {
        return Err("存档版本不匹配".into());
    }
    let mut seed = world.seed;
    let mut blocks_raw = None;
    let mut walls_raw = None;
    let mut inv_raw = None;
    let mut hotbar_raw = None;
    let mut bag_raw = None;
    let mut armor_raw = None;
    let mut accessory_raw = None;
    let mut tdur_raw = None;
    let mut player_raw = None;
    let mut enemies_raw = None;
    let mut meta_raw = None;
    let mut chests_raw = None;
    let mut fluids_raw = None;
    let mut home_raw = None;
    for line in lines {
        if let Some(rest) = line.strip_prefix("seed=") {
            seed = rest.parse().map_err(|_| "seed 无效")?;
        } else if let Some(rest) = line.strip_prefix("meta=") {
            meta_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("player=") {
            player_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("home=") {
            home_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("inv=") {
            inv_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("hotbar=") {
            hotbar_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("bag=") {
            bag_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("armor=") {
            armor_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("accessory=") {
            accessory_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("tdur=") {
            tdur_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("blocks=") {
            blocks_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("walls=") {
            walls_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("chests=") {
            chests_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("fluids=") {
            fluids_raw = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("enemies=") {
            enemies_raw = Some(rest.to_string());
        }
    }
    if seed != world.seed {
        // 同会话种子不一致则整表重建再覆盖方块
        *world = World::generate(seed);
    }
    let blocks = blocks_raw.ok_or("缺 blocks")?;
    if !world.decode_blocks(&blocks) {
        return Err("方块表长度不匹配".into());
    }
    if let Some(walls) = walls_raw {
        let _ = world.decode_walls(&walls);
    }
    if let Some(chests) = chests_raw {
        let _ = world.decode_chests(&chests);
    }
    if let Some(fluids) = fluids_raw {
        let _ = world.decode_fluids(&fluids);
    } else {
        let _ = world.decode_fluids("");
    }
    let prow = player_raw.ok_or("缺 player")?;
    let p: Vec<&str> = prow.split(',').collect();
    if p.len() < 5 {
        return Err("player 字段不足".into());
    }
    player.x = p[0].parse().map_err(|_| "x")?;
    player.y = p[1].parse().map_err(|_| "y")?;
    player.hp = p[2].parse().map_err(|_| "hp")?;
    player.max_hp = p[3].parse().map_err(|_| "max_hp")?;
    player
        .inv
        .select_hotbar(p[4].parse().map_err(|_| "hotbar")?);
    if let Some(f) = p.get(5) {
        player.facing = f.parse().unwrap_or(1.0);
    }
    if let Some(mp) = p.get(7) {
        player.mp = mp.parse().unwrap_or(player.max_mp);
    }
    if let Some(mmp) = p.get(8) {
        player.max_mp = mmp.parse().unwrap_or(player.max_mp);
    }
    player.vx = 0.0;
    player.vy = 0.0;
    player.pending_respawn = false;
    player.home_spawn = None;
    if let Some(h) = home_raw {
        if !h.is_empty() {
            let parts: Vec<&str> = h.split(',').collect();
            if parts.len() >= 2 {
                if let (Ok(hx), Ok(hy)) = (parts[0].parse(), parts[1].parse()) {
                    player.home_spawn = Some((hx, hy));
                }
            }
        }
    }
    player.inv.clear();
    if let (Some(hb), Some(bg)) = (hotbar_raw.as_ref(), bag_raw.as_ref()) {
        player.inv.load_hotbar(hb);
        player.inv.load_bag(bg);
        if let Some(ar) = armor_raw.as_ref() {
            player.inv.load_armor(ar);
        }
        if let Some(ac) = accessory_raw.as_ref() {
            player.inv.load_accessory(ac);
        }
    } else if let Some(inv) = inv_raw {
        // 旧存档：汇总数量灌入
        for part in inv.split(',').filter(|s| !s.is_empty()) {
            let (a, b) = part.split_once(':').ok_or("inv 项")?;
            let id = ItemId(a.parse().map_err(|_| "item id")?);
            let n: u32 = b.parse().map_err(|_| "item n")?;
            let _ = player.inv.add(id, n);
        }
        let _ = tdur_raw; // 旧耐久池已并入堆字段，忽略
    }
    enemies.clear();
    if let Some(eline) = enemies_raw {
        for part in eline.split(';').filter(|s| !s.is_empty()) {
            let f: Vec<&str> = part.split(',').collect();
            if f.len() < 3 {
                continue;
            }
            let x: f32 = f[0].parse().unwrap_or(0.0);
            let y: f32 = f[1].parse().unwrap_or(0.0);
            let hp: f32 = f[2].parse().unwrap_or(36.0);
            let facing: f32 = f.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            let mut e = Enemy::slime(x, y);
            e.hp = hp;
            e.facing = facing;
            if e.hp > 0.0 {
                enemies.push(e);
            }
        }
    }
    if let Some(m) = meta_raw {
        let parts: Vec<&str> = m.split(',').collect();
        if parts.len() >= 4 {
            extra.day_t = parts[0].parse().unwrap_or(extra.day_t);
            extra.seen_warp = parts[1] == "1";
            extra.warped_once = parts[2] == "1";
            if let Some(o) = parse_objective(parts[3]) {
                extra.objective = o;
            }
            if parts.len() >= 5 {
                extra.survived_night = parts[4] == "1";
            }
        }
    }
    Ok(())
}
