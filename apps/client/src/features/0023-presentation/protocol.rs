use super::state::{MAX_PENDING_SOUNDS, SoundCue};
use crate::play::Pushed;
use oteryn_session::{AppliedDelta, PresentationEvent, PresentationEventKind};

pub(in crate::play) fn presentation(
    pushed: &mut Pushed,
    delta: AppliedDelta<Vec<PresentationEvent>>,
) {
    for event in delta.value {
        if matches!(
            event.kind,
            PresentationEventKind::ValueText { .. } | PresentationEventKind::MagicEffect { .. }
        ) {
            absorb_visuals(&mut pushed.visuals, [event.kind]);
        }
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

/// Keep transient visuals bounded even when the shell stops polling.
pub(in crate::play) fn absorb_visuals(
    pending: &mut Vec<PresentationEventKind>,
    newer: impl IntoIterator<Item = PresentationEventKind>,
) {
    for cue in newer {
        pending.push(cue);
        let excess = pending.len().saturating_sub(super::state::MAX_VISUAL_CUES);
        pending.drain(..excess);
    }
}
