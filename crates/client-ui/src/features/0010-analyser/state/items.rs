use super::{Panel, Series};
use std::collections::BTreeMap;

pub const MAX_ITEM_DETAILS: usize = 16_384;
#[derive(Debug, Clone, Default)]
pub struct ItemStats {
    pub count: u64,
    pub history: Series,
}

pub fn record_item(
    items: &mut BTreeMap<u32, ItemStats>,
    item: u32,
    count: u64,
    seconds: u64,
) -> bool {
    if !items.contains_key(&item) && items.len() >= MAX_ITEM_DETAILS {
        return false;
    }
    let row = items.entry(item).or_default();
    row.count = row.count.saturating_add(count);
    row.history.add(seconds, count);
    true
}

pub fn valued_history(
    items: &BTreeMap<u32, ItemStats>,
    prices: &BTreeMap<u32, u64>,
    mut values: Vec<u64>,
    seconds: u64,
) -> Vec<u64> {
    for (item, row) in items {
        if let Some(price) = prices.get(item) {
            for (total, count) in values.iter_mut().zip(row.history.values(seconds)) {
                *total = total.saturating_add(count.saturating_mul(*price));
            }
        }
    }
    values
}
impl Panel {
    pub fn supply_unit_cost(&self, item: u32) -> Option<u64> {
        let info = self.item_info.get(&item)?;
        if info.coin_worth != 0 {
            Some(info.coin_worth)
        } else {
            self.supply_prices.get(&item).copied()
        }
    }
    pub fn supply_cost(&self) -> u64 {
        self.supply.iter().fold(0u64, |total, (item, row)| {
            total.saturating_add(
                row.count
                    .saturating_mul(self.supply_unit_cost(*item).unwrap_or(0)),
            )
        })
    }
    pub fn supply_values(&self, seconds: u64) -> Vec<u64> {
        let prices = self
            .supply
            .keys()
            .filter_map(|item| self.supply_unit_cost(*item).map(|price| (*item, price)))
            .collect();
        valued_history(
            &self.supply,
            &prices,
            Series::default().values(seconds),
            seconds,
        )
    }
    pub fn loot_value(&self) -> u64 {
        self.loot
            .iter()
            .filter(|(id, _)| {
                self.item_info
                    .get(id)
                    .is_some_and(|info| info.coin_worth == 0)
            })
            .fold(self.gold, |total, (id, row)| {
                total.saturating_add(
                    row.count
                        .saturating_mul(self.loot_prices.get(id).copied().unwrap_or(0)),
                )
            })
    }
    pub fn loot_values(&self, seconds: u64) -> Vec<u64> {
        let prices = self
            .loot_prices
            .iter()
            .filter(|(id, _)| {
                self.item_info
                    .get(id)
                    .is_some_and(|info| info.coin_worth == 0)
            })
            .map(|(id, value)| (*id, *value))
            .collect();
        valued_history(
            &self.loot,
            &prices,
            self.gold_chart.values(seconds),
            seconds,
        )
    }
}
