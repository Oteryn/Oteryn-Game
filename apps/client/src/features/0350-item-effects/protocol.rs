use crate::play::Pushed;
use oteryn_session::{AppliedDelta, PresentationEvent};
pub(in crate::play) fn item_effects(
    pushed: &mut Pushed,
    delta: AppliedDelta<Vec<PresentationEvent>>,
) {
    let cues = delta.value.into_iter().filter_map(|event| {
        oteryn_client_ui::item_effects::protocol::cue(event.kind).map(|_| event.kind)
    });
    crate::play::feature_0023_presentation::protocol::absorb_visuals(&mut pushed.visuals, cues);
}
