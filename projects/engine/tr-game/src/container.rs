//! 背包 / 箱子光标：左键拿放互换合并，右键拆半或放一，Shift 快速移动。

use tr_core::ItemId;

use crate::player::{HOTBAR_LEN, Inventory, ItemStack};
use crate::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRef {
    Hotbar(usize),
    Bag(usize),
    /// 木箱列表行（按当前排序后的条目下标）。
    Chest(usize),
}

/// 左键：空指针拾取整堆；有指针则放入 / 合并 / 互换。
pub fn click_left(
    inv: &mut Inventory,
    world: &mut World,
    chest: Option<(i32, i32)>,
    slot: SlotRef,
    cursor: &mut Option<ItemStack>,
) {
    match slot {
        SlotRef::Hotbar(i) | SlotRef::Bag(i) => {
            let cell = slot_mut(inv, slot, i);
            left_on_cell(cursor, cell);
            inv.sync_bag_len();
        }
        SlotRef::Chest(row) => {
            let Some((cx, cy)) = chest else {
                return;
            };
            let Some(map) = world.chest_at(cx, cy) else {
                return;
            };
            let entries = chest_entries(map);
            if row >= entries.len() {
                if let Some(stack) = cursor.take() {
                    *map.entry(stack.id).or_insert(0) += stack.count;
                }
                return;
            }
            let (id, n) = entries[row];
            match cursor.take() {
                None => {
                    if let Some(have) = map.get_mut(&id) {
                        let take = *have;
                        *have = 0;
                        if *have == 0 {
                            map.remove(&id);
                        }
                        *cursor = Some(ItemStack::new(id, take));
                    }
                }
                Some(held) if held.id == id => {
                    *map.entry(id).or_insert(0) += held.count;
                }
                Some(held) => {
                    map.remove(&id);
                    *map.entry(held.id).or_insert(0) += held.count;
                    *cursor = Some(ItemStack::new(id, n));
                }
            }
        }
    }
}

/// 右键：空指针取半；有指针放入 1 个。
pub fn click_right(
    inv: &mut Inventory,
    world: &mut World,
    chest: Option<(i32, i32)>,
    slot: SlotRef,
    cursor: &mut Option<ItemStack>,
) {
    match slot {
        SlotRef::Hotbar(i) | SlotRef::Bag(i) => {
            let cell = slot_mut(inv, slot, i);
            right_on_cell(cursor, cell);
            inv.sync_bag_len();
        }
        SlotRef::Chest(row) => {
            let Some((cx, cy)) = chest else {
                return;
            };
            let Some(map) = world.chest_at(cx, cy) else {
                return;
            };
            if let Some(held) = cursor.as_mut() {
                *map.entry(held.id).or_insert(0) += 1;
                held.count -= 1;
                if held.count == 0 {
                    *cursor = None;
                }
                return;
            }
            let entries = chest_entries(map);
            if row >= entries.len() {
                return;
            }
            let (id, n) = entries[row];
            let half = (n + 1) / 2;
            if let Some(have) = map.get_mut(&id) {
                *have = have.saturating_sub(half);
                if *have == 0 {
                    map.remove(&id);
                }
            }
            *cursor = Some(ItemStack::new(id, half));
        }
    }
}

/// Shift+左键：有箱则与木箱互搬；无箱则快捷栏↔背包。
pub fn click_shift(
    inv: &mut Inventory,
    world: &mut World,
    chest: Option<(i32, i32)>,
    slot: SlotRef,
) {
    match slot {
        SlotRef::Hotbar(i) => {
            let hi = i % HOTBAR_LEN;
            let Some(stack) = inv.hotbar[hi].take() else {
                return;
            };
            if let Some((cx, cy)) = chest {
                if let Some(map) = world.chest_at(cx, cy) {
                    *map.entry(stack.id).or_insert(0) += stack.count;
                } else {
                    inv.hotbar[hi] = Some(stack);
                }
            } else {
                let id = stack.id;
                let left = inv.add(id, stack.count);
                if left > 0 {
                    inv.hotbar[hi] = Some(ItemStack::new(id, left));
                }
            }
            inv.sync_bag_len();
        }
        SlotRef::Bag(i) => {
            if i >= inv.bag.len() {
                return;
            }
            let Some(stack) = inv.bag[i].take() else {
                return;
            };
            if let Some((cx, cy)) = chest {
                if let Some(map) = world.chest_at(cx, cy) {
                    *map.entry(stack.id).or_insert(0) += stack.count;
                } else {
                    inv.bag[i] = Some(stack);
                }
            } else if let Some(hi) = inv.hotbar.iter().position(|s| s.is_none()) {
                inv.hotbar[hi] = Some(stack);
            } else {
                inv.bag[i] = Some(stack);
            }
            inv.sync_bag_len();
        }
        SlotRef::Chest(row) => {
            let Some((cx, cy)) = chest else {
                return;
            };
            let Some(map) = world.chest_at(cx, cy) else {
                return;
            };
            let entries = chest_entries(map);
            if row >= entries.len() {
                return;
            }
            let (id, n) = entries[row];
            map.remove(&id);
            let left = inv.add(id, n);
            if left > 0 {
                if let Some(map) = world.chest_at(cx, cy) {
                    *map.entry(id).or_insert(0) += left;
                }
            }
            inv.sync_bag_len();
        }
    }
}

/// 关闭面板时把光标物品塞回背包；仍装不下则返回残留供世界掉落。
pub fn absorb_cursor(inv: &mut Inventory, cursor: &mut Option<ItemStack>) -> Option<ItemStack> {
    let Some(stack) = cursor.take() else {
        return None;
    };
    let left = inv.add(stack.id, stack.count);
    if left > 0 {
        Some(ItemStack::new(stack.id, left))
    } else {
        None
    }
}

pub fn chest_entries(map: &std::collections::HashMap<ItemId, u32>) -> Vec<(ItemId, u32)> {
    let mut v: Vec<_> = map
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(a, b)| (*a, *b))
        .collect();
    v.sort_by_key(|(id, _)| id.0);
    v
}

fn slot_mut(inv: &mut Inventory, slot: SlotRef, i: usize) -> &mut Option<ItemStack> {
    match slot {
        SlotRef::Hotbar(_) => {
            let hi = i % HOTBAR_LEN;
            &mut inv.hotbar[hi]
        }
        SlotRef::Bag(_) => {
            if i >= inv.bag.len() {
                inv.bag.resize(i + 1, None);
            }
            &mut inv.bag[i]
        }
        SlotRef::Chest(_) => unreachable!("chest handled separately"),
    }
}

fn left_on_cell(cursor: &mut Option<ItemStack>, cell: &mut Option<ItemStack>) {
    match (cursor.take(), cell.take()) {
        (None, None) => {}
        (None, Some(s)) => *cursor = Some(s),
        (Some(c), None) => *cell = Some(c),
        (Some(c), Some(s)) if can_merge(&c, &s) => {
            let mut merged = s;
            merged.count = merged.count.saturating_add(c.count);
            *cell = Some(merged);
        }
        (Some(c), Some(s)) => {
            *cell = Some(c);
            *cursor = Some(s);
        }
    }
}

fn right_on_cell(cursor: &mut Option<ItemStack>, cell: &mut Option<ItemStack>) {
    match (cursor.as_mut(), cell.as_mut()) {
        (None, Some(s)) => {
            if s.count <= 1 {
                *cursor = cell.take();
                return;
            }
            let half = (s.count + 1) / 2;
            s.count -= half;
            *cursor = Some(ItemStack::new(s.id, half));
        }
        (Some(c), None) => {
            *cell = Some(ItemStack::new(c.id, 1));
            c.count -= 1;
            if c.count == 0 {
                *cursor = None;
            }
        }
        (Some(c), Some(s)) if can_merge(c, s) => {
            s.count = s.count.saturating_add(1);
            c.count -= 1;
            if c.count == 0 {
                *cursor = None;
            }
        }
        _ => {}
    }
}

fn can_merge(a: &ItemStack, b: &ItemStack) -> bool {
    a.id == b.id && !a.id.is_tool() && a.id.weapon().is_none() && !a.id.is_accessory()
}
