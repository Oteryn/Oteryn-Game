//! Reserve Oberon's physical phase on the same native think path as Gaz.
use super::*;
use crate::map::boot::{BundleCollisionIndexExt, BundleMap};
use crate::map::facts::MovementSpeech;
use oteryn_protocol_oteryn::entity_names::EntityDisplayName;

pub(super) async fn think(
    runtime: &mut ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[crate::foundation::ThinkOccurrence],
) -> Result<(), ()> {
    for occurrence in occurrences {
        let actor = occurrence.actor;
        if !runtime.oberon_phase_due(*occurrence) {
            continue;
        }
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
        else {
            return Err(());
        };
        let at = runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position();
        if map.read(runtime, at).await.is_none() || !map.validate_transaction(runtime).await {
            return Err(());
        }
        let digest = Sha256::new()
            .chain_update(actor.placement_identity())
            .chain_update(actor.actor_local_generation().to_be_bytes())
            .chain_update(occurrence.sequence.to_be_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let root = GameplayDecisionRoot::from_bytes(native.source_digest());
        let mut ordinal = 0;
        let phase = runtime
            .think_oberon(
                *occurrence,
                native.source_digest(),
                |bound| {
                    let draw = deterministic_decision_u64(
                        &root,
                        DecisionOccurrenceId::from_bytes(id),
                        "oberon_debate",
                        ordinal,
                    );
                    ordinal += 1;
                    draw.map_or(bound, |draw| {
                        crate::spell::uniform_draw(draw, 0, i64::from(bound - 1)) as u32
                    })
                },
                |runtime, at| map.source_tile_exists(runtime, at),
            )
            .map_err(|_| ())?;
        if let Some(taunt) = phase {
            say(runtime, index.map(), actor, taunt);
        }
    }
    Ok(())
}

pub(in crate::gameplay_transport) fn say(
    runtime: &ChannelRuntimeV1,
    map: &BundleMap,
    actor: ExactActorRef,
    text: &'static str,
) {
    let Some(position) = runtime
        .read_actor_position(actor)
        .ok()
        .and_then(|at| visual_position(at.position()))
    else {
        return;
    };
    let Ok(name) = EntityDisplayName::new("Grand Master Oberon") else {
        return;
    };
    map.facts.emit_movement_speech(MovementSpeech {
        speaker: actor,
        position,
        talk_type: 36,
        text: text.into(),
        captured_name: Some(name),
    });
}
