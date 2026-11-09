//! Declare only the additional owned distinct-target state; legacy tracks stay intact.
use super::{QUEST, TARGETS};
use crate::quest::{QuestTrack, native_rules::NativeRule, world_callbacks::Callback};

pub(crate) fn declare(tracks: &mut Vec<QuestTrack>, callbacks: &[Callback]) -> Result<(), String> {
    if !callbacks
        .iter()
        .any(|callback| matches!(callback.native_rule, Some(NativeRule::IceSealPaint { .. })))
    {
        return Ok(());
    }
    let expected = QuestTrack {
        key: TARGETS.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: (1_i64 << 48) - 1,
    };
    if let Some(existing) = tracks.iter().find(|track| track.key == TARGETS) {
        if existing != &expected {
            return Err(
                "seal target counter has incompatible ownership, initial value or bounds".into(),
            );
        }
    } else {
        tracks.push(expected);
    }
    Ok(())
}
