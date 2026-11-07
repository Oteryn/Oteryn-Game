use oteryn_client_runtime::item_effects::{ActorPosition, PresentationEventKind};
pub fn cue(kind: PresentationEventKind) -> Option<(ActorPosition, u32)> {
    match kind {
        PresentationEventKind::MagicEffect {
            position,
            effect_id,
            ..
        } => Some((position, effect_id)),
        _ => None,
    }
}
