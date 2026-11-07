use super::protocol::cue;
use oteryn_client_runtime::item_effects::{
    ActorPosition, PresentationEventKind, PresentationSource,
};
#[test]
fn item_effects_reject_non_item_visual_data() {
    let position = ActorPosition {
        x: 1,
        y: 2,
        floor: 7,
    };
    assert_eq!(
        cue(PresentationEventKind::MagicEffect {
            position,
            effect_id: 29,
            source: PresentationSource::Own
        }),
        Some((position, 29))
    );
    assert_eq!(
        cue(PresentationEventKind::Sound {
            position,
            cue_id: 29,
            secondary_cue_id: 0,
            source: PresentationSource::Own
        }),
        None
    );
    assert_eq!(cue(PresentationEventKind::AmbientSound { cue_id: 1 }), None);
}
