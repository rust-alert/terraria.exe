//! 装备（快捷栏 / 护甲）与背包（容量随背包道具变化）。

use tr_core::ItemId;

/// 快捷栏固定格数。
pub const HOTBAR_LEN: usize = 10;
/// 无背包道具时的口袋底数。
pub const BAG_POCKET_SLOTS: usize = 8;

/// 单格物品堆。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub id: ItemId,
    pub count: u32,
    /// 工具耐久池；非工具为 0。
    pub dur: u16,
}

impl ItemStack {
    pub fn new(id: ItemId, count: u32) -> Self {
        let dur = if id.is_tool() {
            id.max_durability().saturating_mul(count as u16)
        } else {
            0
        };
        Self { id, count, dur }
    }

    pub fn tool_dur_left(&self) -> Option<(u16, u16)> {
        if !self.id.is_tool() || self.count == 0 {
            return None;
        }
        let max = self.id.max_durability();
        let cur = (self.dur / self.count.max(1) as u16).min(max);
        Some((cur, max))
    }
}

/// 玩家物品：装备栏与背包分离。
#[derive(Debug, Clone)]
pub struct Inventory {
    pub hotbar: [Option<ItemStack>; HOTBAR_LEN],
    pub hotbar_sel: usize,
    pub armor: Option<ItemStack>,
    /// 饰品槽（当前单格）。
    pub accessory: Option<ItemStack>,
    pub bag: Vec<Option<ItemStack>>,
}

impl Default for Inventory {
    fn default() -> Self {
        let mut inv = Self {
            hotbar: std::array::from_fn(|_| None),
            hotbar_sel: 0,
            armor: None,
            accessory: None,
            bag: Vec::new(),
        };
        inv.sync_bag_len();
        inv
    }
}

impl Inventory {
    /// 当前背包容量（口袋 + 持有背包道具加成）。
    pub fn bag_capacity(&self) -> usize {
        let mut bonus = 0u32;
        for slot in self.hotbar.iter().chain(self.bag.iter()) {
            if let Some(stack) = slot {
                bonus =
                    bonus.saturating_add(stack.id.bag_bonus_slots().saturating_mul(stack.count));
            }
        }
        BAG_POCKET_SLOTS.saturating_add(bonus as usize)
    }

    pub fn sync_bag_len(&mut self) {
        let cap = self.bag_capacity();
        if self.bag.len() < cap {
            self.bag.resize(cap, None);
        } else if self.bag.len() > cap {
            // 只在尾部空格时收缩；有物则保持，等腾空后再缩。
            while self.bag.len() > cap && self.bag.last().is_some_and(|s| s.is_none()) {
                self.bag.pop();
            }
        }
    }

    pub fn used_bag_slots(&self) -> usize {
        self.bag.iter().filter(|s| s.is_some()).count()
    }

    pub fn get(&self, id: ItemId) -> u32 {
        let mut n = 0u32;
        for slot in self.hotbar.iter().chain(self.bag.iter()) {
            if let Some(s) = slot {
                if s.id == id {
                    n = n.saturating_add(s.count);
                }
            }
        }
        if let Some(a) = &self.armor {
            if a.id == id {
                n = n.saturating_add(a.count);
            }
        }
        if let Some(a) = &self.accessory {
            if a.id == id {
                n = n.saturating_add(a.count);
            }
        }
        n
    }

    pub fn selected_stack(&self) -> Option<&ItemStack> {
        let i = self.hotbar_sel % HOTBAR_LEN;
        self.hotbar[i].as_ref()
    }

    pub fn selected_item(&self) -> Option<ItemId> {
        self.selected_stack().map(|s| s.id)
    }

    pub fn select_hotbar(&mut self, i: usize) {
        self.hotbar_sel = i % HOTBAR_LEN;
    }

    /// 选中含该物品的快捷栏格；若仅在背包则换入空快捷栏格。
    pub fn select_first(&mut self, id: ItemId) -> bool {
        if let Some(i) = self
            .hotbar
            .iter()
            .position(|s| s.as_ref().is_some_and(|x| x.id == id))
        {
            self.hotbar_sel = i;
            return true;
        }
        if let Some(bi) = self
            .bag
            .iter()
            .position(|s| s.as_ref().is_some_and(|x| x.id == id))
        {
            let hi = self
                .hotbar
                .iter()
                .position(|s| s.is_none())
                .unwrap_or(self.hotbar_sel % HOTBAR_LEN);
            self.swap_hotbar_bag(hi, bi);
            self.hotbar_sel = hi;
            return true;
        }
        false
    }

    /// 尝试加入物品；返回未能装下的数量。
    pub fn add(&mut self, id: ItemId, mut n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.sync_bag_len();
        n = self.merge_into_existing(id, n);
        if n == 0 {
            self.sync_bag_len();
            return 0;
        }
        // 工具优先空快捷栏，材料优先空背包。
        if id.is_tool() || id.weapon().is_some() {
            n = fill_empty_slots(&mut self.hotbar, id, n);
            self.sync_bag_len();
            n = fill_empty_slots(&mut self.bag, id, n);
        } else {
            self.sync_bag_len();
            n = fill_empty_slots(&mut self.bag, id, n);
            n = fill_empty_slots(&mut self.hotbar, id, n);
        }
        self.sync_bag_len();
        n
    }

    fn merge_into_existing(&mut self, id: ItemId, mut n: u32) -> u32 {
        for slot in self.hotbar.iter_mut().chain(self.bag.iter_mut()) {
            if n == 0 {
                break;
            }
            if let Some(s) = slot {
                if s.id == id {
                    s.count = s.count.saturating_add(n);
                    if id.is_tool() {
                        s.dur = s
                            .dur
                            .saturating_add(id.max_durability().saturating_mul(n as u16));
                    }
                    n = 0;
                }
            }
        }
        n
    }

    pub fn try_take(&mut self, id: ItemId, mut n: u32) -> bool {
        if self.get(id) < n {
            return false;
        }
        // 先背包后快捷栏，护甲最后（一般不从护甲扣材料）。
        n = Self::take_from_slots(&mut self.bag, id, n);
        n = Self::take_from_slots(&mut self.hotbar, id, n);
        if n > 0 {
            if let Some(a) = &mut self.armor {
                if a.id == id {
                    let use_n = n.min(a.count);
                    a.count -= use_n;
                    n -= use_n;
                    if a.count == 0 {
                        self.armor = None;
                    }
                }
            }
        }
        if n > 0 {
            if let Some(a) = &mut self.accessory {
                if a.id == id {
                    let use_n = n.min(a.count);
                    a.count -= use_n;
                    n -= use_n;
                    if a.count == 0 {
                        self.accessory = None;
                    }
                }
            }
        }
        debug_assert!(n == 0);
        self.sync_bag_len();
        true
    }

    fn take_from_slots(slots: &mut [Option<ItemStack>], id: ItemId, mut n: u32) -> u32 {
        for slot in slots.iter_mut() {
            if n == 0 {
                break;
            }
            let clear = if let Some(s) = slot.as_mut() {
                if s.id == id {
                    let use_n = n.min(s.count);
                    if s.id.is_tool() && s.count > 0 {
                        let per = s.dur / s.count.max(1) as u16;
                        s.dur = s.dur.saturating_sub(per.saturating_mul(use_n as u16));
                    }
                    s.count -= use_n;
                    n -= use_n;
                    s.count == 0
                } else {
                    false
                }
            } else {
                false
            };
            if clear {
                *slot = None;
            }
        }
        n
    }

    pub fn tool_dur_left(&self, id: ItemId) -> Option<(u16, u16)> {
        let mut pool = 0u16;
        let mut count = 0u32;
        for slot in self.hotbar.iter().chain(self.bag.iter()) {
            if let Some(s) = slot {
                if s.id == id && id.is_tool() {
                    pool = pool.saturating_add(s.dur);
                    count = count.saturating_add(s.count);
                }
            }
        }
        if count == 0 {
            return None;
        }
        let max = id.max_durability();
        Some(((pool / count as u16).min(max), max))
    }

    pub fn wear_tool(&mut self, id: ItemId, amount: u16) -> Option<String> {
        if !id.is_tool() || amount == 0 {
            return None;
        }
        // 优先手持格。
        let sel = self.hotbar_sel % HOTBAR_LEN;
        if let Some(s) = self.hotbar[sel].as_mut() {
            if s.id == id {
                return Self::wear_stack(s, amount).map(|broke| {
                    if broke {
                        self.hotbar[sel] = None;
                    }
                    format!("{} 损坏了", id.label())
                });
            }
        }
        for slot in self.hotbar.iter_mut().chain(self.bag.iter_mut()) {
            if let Some(s) = slot {
                if s.id == id {
                    if let Some(broke) = Self::wear_stack(s, amount) {
                        if broke {
                            *slot = None;
                        }
                        return Some(format!("{} 损坏了", id.label()));
                    }
                    return None;
                }
            }
        }
        None
    }

    /// 返回 `Some(true)` 表示整件损坏需清空格；`None` 表示未磨损到碎。
    fn wear_stack(s: &mut ItemStack, amount: u16) -> Option<bool> {
        if s.dur <= amount {
            // 碎一件
            if s.count <= 1 {
                return Some(true);
            }
            s.count -= 1;
            s.dur = s.id.max_durability().saturating_mul(s.count as u16);
            return Some(false); // 碎了一件但堆还在，仍提示
        }
        s.dur -= amount;
        None
    }

    pub fn can_pay(&self, inputs: &[(ItemId, u32)]) -> bool {
        inputs.iter().all(|(id, n)| self.get(*id) >= *n)
    }

    pub fn pay(&mut self, inputs: &[(ItemId, u32)]) -> bool {
        if !self.can_pay(inputs) {
            return false;
        }
        for (id, n) in inputs {
            let _ = self.try_take(*id, *n);
        }
        true
    }

    pub fn clear(&mut self) {
        self.hotbar = std::array::from_fn(|_| None);
        self.armor = None;
        self.accessory = None;
        self.bag.clear();
        self.sync_bag_len();
    }

    /// 穿上护甲（从总数中扣 1 件放到护甲槽）。
    pub fn equip_armor(&mut self, id: ItemId) -> Option<String> {
        if self.get(id) == 0 {
            return Some("没有该护甲".into());
        }
        if let Some(old) = self.armor.take() {
            let left = self.add(old.id, old.count);
            if left > 0 {
                // 装不下旧甲则穿回
                self.armor = Some(old);
                return Some("背包已满，无法替换护甲".into());
            }
        }
        if !self.try_take(id, 1) {
            return Some("没有该护甲".into());
        }
        self.armor = Some(ItemStack::new(id, 1));
        Some(format!("已装备 {}", id.label()))
    }

    pub fn has_armor(&self) -> bool {
        self.armor
            .as_ref()
            .is_some_and(|a| a.id == ItemId::WOOD_ARMOR)
    }

    /// 装备饰品到饰品槽。
    pub fn equip_accessory(&mut self, id: ItemId) -> Option<String> {
        if !id.is_accessory() {
            return Some("不是饰品".into());
        }
        if self.get(id) == 0 {
            return Some(format!("没有{}", id.label()));
        }
        if let Some(old) = self.accessory.take() {
            let left = self.add(old.id, old.count);
            if left > 0 {
                self.accessory = Some(old);
                return Some("背包已满，无法替换饰品".into());
            }
        }
        if !self.try_take(id, 1) {
            return Some(format!("没有{}", id.label()));
        }
        self.accessory = Some(ItemStack::new(id, 1));
        Some(format!("已装备 {}", id.label()))
    }

    pub fn has_cloud_jump(&self) -> bool {
        self.accessory
            .as_ref()
            .is_some_and(|a| a.id == ItemId::CLOUD_BOTTLE)
    }

    /// 快捷栏与背包格互换。
    pub fn swap_hotbar_bag(&mut self, hot_i: usize, bag_i: usize) {
        let hi = hot_i % HOTBAR_LEN;
        if bag_i >= self.bag.len() {
            return;
        }
        let tmp = self.hotbar[hi].take();
        self.hotbar[hi] = self.bag[bag_i].take();
        self.bag[bag_i] = tmp;
        self.sync_bag_len();
    }

    pub fn entries(&self) -> Vec<(ItemId, u32)> {
        let mut map = std::collections::BTreeMap::new();
        for slot in self.hotbar.iter().chain(self.bag.iter()) {
            if let Some(s) = slot {
                *map.entry(s.id.0).or_insert(0u32) += s.count;
            }
        }
        if let Some(a) = &self.armor {
            *map.entry(a.id.0).or_insert(0u32) += a.count;
        }
        if let Some(a) = &self.accessory {
            *map.entry(a.id.0).or_insert(0u32) += a.count;
        }
        map.into_iter().map(|(id, n)| (ItemId(id), n)).collect()
    }

    pub fn summary_line(&self) -> String {
        self.entries()
            .into_iter()
            .map(|(id, n)| format!("{} {}", id.label(), n))
            .collect::<Vec<_>>()
            .join("  ")
    }

    /// 存档：快捷栏。
    pub fn encode_hotbar(&self) -> String {
        self.hotbar
            .iter()
            .map(|s| match s {
                None => "-".into(),
                Some(s) => format!("{}:{}:{}", s.id.0, s.count, s.dur),
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn encode_bag(&self) -> String {
        self.bag
            .iter()
            .map(|s| match s {
                None => "-".into(),
                Some(s) => format!("{}:{}:{}", s.id.0, s.count, s.dur),
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn encode_armor(&self) -> String {
        match &self.armor {
            None => String::new(),
            Some(s) => format!("{}:{}:{}", s.id.0, s.count, s.dur),
        }
    }

    pub fn encode_accessory(&self) -> String {
        match &self.accessory {
            None => String::new(),
            Some(s) => format!("{}:{}:{}", s.id.0, s.count, s.dur),
        }
    }

    pub fn load_hotbar(&mut self, raw: &str) {
        self.hotbar = std::array::from_fn(|_| None);
        for (i, part) in raw.split(',').enumerate() {
            if i >= HOTBAR_LEN {
                break;
            }
            self.hotbar[i] = parse_stack(part);
        }
    }

    pub fn load_bag(&mut self, raw: &str) {
        self.bag.clear();
        for part in raw.split(',').filter(|s| !s.is_empty()) {
            self.bag.push(parse_stack(part));
        }
        self.sync_bag_len();
    }

    pub fn load_armor(&mut self, raw: &str) {
        self.armor = if raw.is_empty() {
            None
        } else {
            parse_stack(raw)
        };
    }

    pub fn load_accessory(&mut self, raw: &str) {
        self.accessory = if raw.is_empty() {
            None
        } else {
            parse_stack(raw)
        };
    }
}

fn fill_empty_slots(slots: &mut [Option<ItemStack>], id: ItemId, mut n: u32) -> u32 {
    for slot in slots.iter_mut() {
        if n == 0 {
            break;
        }
        if slot.is_none() {
            *slot = Some(ItemStack::new(id, n));
            n = 0;
        }
    }
    n
}

fn parse_stack(part: &str) -> Option<ItemStack> {
    if part == "-" || part.is_empty() {
        return None;
    }
    let mut it = part.split(':');
    let id = ItemId(it.next()?.parse().ok()?);
    let count: u32 = it.next()?.parse().ok()?;
    let dur: u16 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    if count == 0 {
        return None;
    }
    Some(ItemStack { id, count, dur })
}
