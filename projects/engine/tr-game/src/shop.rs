//! 商人商店：用铜币购买常用物。图标走正版 `Item_N`。

use tr_core::ItemId;

use crate::player::Inventory;

/// 一条商品。
#[derive(Debug, Clone, Copy)]
pub struct ShopOffer {
    pub item: ItemId,
    pub count: u32,
    pub price: u32,
    pub label: &'static str,
}

/// 商人货架（竖切常用补给）。
pub const MERCHANT_OFFERS: &[ShopOffer] = &[
    ShopOffer {
        item: ItemId::TORCH,
        count: 10,
        price: 8,
        label: "火把 ×10",
    },
    ShopOffer {
        item: ItemId::ROPE,
        count: 20,
        price: 10,
        label: "绳索 ×20",
    },
    ShopOffer {
        item: ItemId::WOOD_WALL,
        count: 20,
        price: 12,
        label: "木墙 ×20",
    },
    ShopOffer {
        item: ItemId::PLATFORM,
        count: 10,
        price: 6,
        label: "木平台 ×10",
    },
    ShopOffer {
        item: ItemId::LADDER,
        count: 8,
        price: 8,
        label: "木梯 ×8",
    },
    ShopOffer {
        item: ItemId::WOOD_ARROW,
        count: 25,
        price: 15,
        label: "木箭 ×25",
    },
];

/// 尝试购买。成功返回提示，失败返回原因。
pub fn try_buy(inv: &mut Inventory, offer_i: usize) -> Result<String, String> {
    let offer = MERCHANT_OFFERS
        .get(offer_i)
        .ok_or_else(|| "没有这件货".to_string())?;
    let coins = inv.get(ItemId::COPPER_COIN);
    if coins < offer.price {
        return Err(format!(
            "铜币不足（需要 {}，现有 {}）",
            offer.price, coins
        ));
    }
    if !inv.try_take(ItemId::COPPER_COIN, offer.price) {
        return Err("扣款失败".into());
    }
    inv.add(offer.item, offer.count);
    Ok(format!("购入 {}（−{} 铜币）", offer.label, offer.price))
}
