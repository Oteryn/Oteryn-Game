use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::cyclopedia_character::{
    AugmentIntent, CyclopediaCharacterPanel, augment_request,
};
pub(super) fn submit(
    p: &mut CyclopediaCharacterPanel,
    id: usize,
    target: &str,
) -> Option<FeatureCommand> {
    let mut intent = match id {
        180 => AugmentIntent::Refresh,
        181 => AugmentIntent::Sources(7),
        182 => AugmentIntent::Sources(1),
        183 => AugmentIntent::Sources(2),
        184 => AugmentIntent::Sources(4),
        185 => AugmentIntent::Sort,
        112 => AugmentIntent::Previous,
        113 => AugmentIntent::Next,
        _ => return None,
    };
    let target = if target.trim().is_empty() {
        None
    } else {
        let Some(target) = super::target::parse_target(target.trim()) else {
            p.feedback = Some("Enter a valid target UUID");
            return None;
        };
        Some(target)
    };
    if target != p.target && matches!(id, 112 | 113) {
        intent = AugmentIntent::Refresh;
    }
    p.change_target(target);
    augment_request(p, intent)
}
