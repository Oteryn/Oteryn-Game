use super::state::{MAX_PENDING_SOUNDS, SoundCue};
use crate::play::Pushed;
use oteryn_session::{AppliedDelta, PresentationEvent, PresentationEventKind};

pub(in crate::play) fn presentation(
    pushed: &mut Pushed,
    delta: AppliedDelta<Vec<PresentationEvent>>,
) {
    for event in delta.value {
        if let PresentationEventKind::Sound {
            cue_id,
            source,
            secondary_cue_id,
            ..
        } = event.kind
        {
            for id in [cue_id, secondary_cue_id] {
                if id != 0 && pushed.sounds.len() < MAX_PENDING_SOUNDS {
                    pushed.sounds.push(SoundCue { id, source });
                }
            }
        }
    }
}
