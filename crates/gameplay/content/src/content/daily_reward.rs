//! The small server DAILY catalogue. Original reward definitions are admitted here;
//! potion definitions must independently exist in the active native generation.
use super::native_gameplay::NativeGameplayState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyRewardContent {
    pub schema: String,
    pub revision: String,
    pub entry_shrine: DailyRewardShrine,
    pub shrine_source_ids: Vec<u32>,
    pub potion_source_ids: Vec<u32>,
    pub items: Vec<DailyRewardItem>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyRewardShrine {
    pub placement: String,
    pub coordinate_frame: String,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyRewardItem {
    pub key: String,
    pub charges: u32,
    pub kind: String,
}
impl DailyRewardContent {
    pub fn load(native: &NativeGameplayState) -> Option<Self> {
        let content: Self = serde_json::from_str(include_str!(
            "../../../../../content/interactions/daily-rewards.json"
        ))
        .ok()?;
        (content.schema == "OTERYN_DAILY_REWARDS/v1"
            && content.potion_source_ids.iter().all(|id| {
                native
                    .item_policy_for_source_id(*id)
                    .is_some_and(|p| p.record().admission.materializable)
            })
            && content.items.len() == 9
            && content.items.iter().all(|item| match item.kind.as_str() {
                "converter" => item.charges == 100,
                "training" => item.charges == 50,
                _ => false,
            }))
        .then_some(content)
    }
}
