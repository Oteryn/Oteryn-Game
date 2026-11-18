use super::*;
use oteryn_gameplay_encounters::bosses::{
    phase_controller::oberon, source_callbacks::callback_speech::CallbackSpeech,
};

pub(super) fn think(
    runtime: &mut ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    occurrences: &[crate::foundation::ThinkOccurrence],
) {
    for occurrence in occurrences {
        let mut draw_index = 0_u8;
        let draw = |limit| {
            let digest = Sha256::new()
                .chain_update(b"oberon-phase-v1")
                .chain_update(occurrence.actor.placement_identity())
                .chain_update(occurrence.sequence.to_be_bytes())
                .chain_update([draw_index])
                .finalize();
            draw_index += 1;
            u32::from(u16::from_be_bytes([digest[0], digest[1]])) % limit
        };
        if let Ok(Some(taunt)) = oberon::think(runtime, occurrence.actor, occurrence.sequence, draw)
            && let Ok(line) = CallbackSpeech::oberon(runtime, occurrence.actor, taunt)
        {
            spawns::emit_speech(room, vec![line]);
        }
    }
}
