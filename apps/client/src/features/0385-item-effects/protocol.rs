use crate::play::Pushed;
use oteryn_session::{AppliedDelta, PresentationEvent, PresentationEventKind};
pub(in crate::play) fn item_effects(
    pushed: &mut Pushed,
    delta: AppliedDelta<Vec<PresentationEvent>>,
) {
    let cues = delta.value.into_iter().filter_map(|event| {
        oteryn_client_ui::item_effects::protocol::cue(event.kind)?;
        // Presentation's wire ABI carries Tibia floors 0..=15; native map
        // coordinates use their negation. Convert exactly at this new adapter.
        match event.kind {
            PresentationEventKind::MagicEffect {
                mut position,
                effect_id,
                source,
            } => {
                position.floor = position.floor.checked_neg()?;
                Some(PresentationEventKind::MagicEffect {
                    position,
                    effect_id,
                    source,
                })
            }
            _ => None,
        }
    });
    crate::play::feature_0023_presentation::protocol::absorb_visuals(&mut pushed.visuals, cues);
}
