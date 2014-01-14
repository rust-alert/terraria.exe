//! 地表成长目标链：先住所与矿，裂痕传送放在后段。

use tr_core::{BlockId, ItemId};

use crate::player::Player;
use crate::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    GatherWood,
    CraftWorkbench,
    PlaceWorkbench,
    CraftPick,
    PlaceBed,
    SurviveNight,
    CraftStonePick,
    MineCopper,
    FindWarp,
    WarpOnce,
    Done,
}

impl Objective {
    pub fn title(self) -> &'static str {
        match self {
            Self::GatherWood => "收集木材 ×10（砍树，走近拾取）",
            Self::CraftWorkbench => "打开制作，徒手合成工作台",
            Self::PlaceWorkbench => "选中工作台，左键放置",
            Self::CraftPick => "靠近工作台，制作木镐",
            Self::PlaceBed => "制作并放置木床，建立第一个据点",
            Self::SurviveNight => "入夜后在床或逃生舱按 E 睡过一夜",
            Self::CraftStonePick => "靠近工作台，用木材和石头制作石镐",
            Self::MineCopper => "持石镐下到浅层洞穴，采集铜矿 ×3",
            Self::FindWarp => "沿地表找到紫色裂痕异常区",
            Self::WarpOnce => "靠近裂痕锚按 E，完成一次传送",
            Self::Done => "地表起步完成 — 继续探索与成长",
        }
    }

    /// HUD 任务卡短标题。
    pub fn quest_name(self) -> &'static str {
        match self {
            Self::Done => "自由探索",
            Self::FindWarp | Self::WarpOnce => "裂痕信号",
            _ => "地表起步",
        }
    }

    fn order(self) -> u8 {
        match self {
            Self::GatherWood => 0,
            Self::CraftWorkbench => 1,
            Self::PlaceWorkbench => 2,
            Self::CraftPick => 3,
            Self::PlaceBed => 4,
            Self::SurviveNight => 5,
            Self::CraftStonePick => 6,
            Self::MineCopper => 7,
            Self::FindWarp => 8,
            Self::WarpOnce => 9,
            Self::Done => 10,
        }
    }

    fn short(self) -> &'static str {
        match self {
            Self::GatherWood => "收集木材 ×10",
            Self::CraftWorkbench => "制作工作台",
            Self::PlaceWorkbench => "放置工作台",
            Self::CraftPick => "制作木镐",
            Self::PlaceBed => "放置木床",
            Self::SurviveNight => "睡过一夜",
            Self::CraftStonePick => "制作石镐",
            Self::MineCopper => "采集铜矿",
            Self::FindWarp => "发现裂痕",
            Self::WarpOnce => "完成传送",
            Self::Done => "起步完成",
        }
    }

    /// 任务清单：`(文案, 已完成, 当前)`，最多展示当前及前后各一项。
    pub fn checklist(self) -> Vec<(&'static str, bool, bool)> {
        const STEPS: [Objective; 10] = [
            Objective::GatherWood,
            Objective::CraftWorkbench,
            Objective::PlaceWorkbench,
            Objective::CraftPick,
            Objective::PlaceBed,
            Objective::SurviveNight,
            Objective::CraftStonePick,
            Objective::MineCopper,
            Objective::FindWarp,
            Objective::WarpOnce,
        ];
        if self == Self::Done {
            return vec![("地表起步完成", true, true)];
        }
        let cur = self.order();
        STEPS
            .iter()
            .filter(|s| {
                let o = s.order();
                o + 1 >= cur && o <= cur + 1
            })
            .map(|s| {
                let o = s.order();
                (s.short(), o < cur, o == cur)
            })
            .collect()
    }

    pub fn advance(
        self,
        player: &Player,
        world: &World,
        warped: bool,
        seen_warp: bool,
        survived_night: bool,
    ) -> Self {
        let mut cur = self;
        for _ in 0..12 {
            let next = cur.step(player, world, warped, seen_warp, survived_night);
            if next == cur {
                break;
            }
            cur = next;
        }
        cur
    }

    fn step(
        self,
        player: &Player,
        world: &World,
        warped: bool,
        seen_warp: bool,
        survived_night: bool,
    ) -> Self {
        match self {
            Self::GatherWood => {
                if player.inv.get(ItemId::WOOD) >= 10 || player.inv.get(ItemId::WORKBENCH) > 0 {
                    Self::CraftWorkbench
                } else {
                    self
                }
            }
            Self::CraftWorkbench => {
                if player.inv.get(ItemId::WORKBENCH) > 0 || world.has_block(BlockId::WORKBENCH) {
                    Self::PlaceWorkbench
                } else {
                    self
                }
            }
            Self::PlaceWorkbench => {
                if world.has_block(BlockId::WORKBENCH) {
                    Self::CraftPick
                } else {
                    self
                }
            }
            Self::CraftPick => {
                if player.inv.get(ItemId::WOOD_PICK) > 0 || player.inv.get(ItemId::STONE_PICK) > 0 {
                    Self::PlaceBed
                } else {
                    self
                }
            }
            Self::PlaceBed => {
                if world.has_block(BlockId::BED) || player.home_spawn.is_some() {
                    Self::SurviveNight
                } else {
                    self
                }
            }
            Self::SurviveNight => {
                if survived_night {
                    Self::CraftStonePick
                } else {
                    self
                }
            }
            Self::CraftStonePick => {
                if player.inv.get(ItemId::STONE_PICK) > 0
                    || player.inv.get(ItemId::COPPER_PICK) > 0
                    || player.inv.get(ItemId::COPPER_ORE) >= 3
                {
                    Self::MineCopper
                } else {
                    self
                }
            }
            Self::MineCopper => {
                if player.inv.get(ItemId::COPPER_ORE) >= 3
                    || player.inv.get(ItemId::COPPER_BAR) > 0
                    || player.inv.get(ItemId::COPPER_PICK) > 0
                {
                    Self::FindWarp
                } else {
                    self
                }
            }
            Self::FindWarp => {
                if seen_warp {
                    Self::WarpOnce
                } else {
                    self
                }
            }
            Self::WarpOnce => {
                if warped {
                    Self::Done
                } else {
                    self
                }
            }
            Self::Done => Self::Done,
        }
    }
}
