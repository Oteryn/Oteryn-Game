//! Crystal201189f src/creatures/creature.cpp224-260: standard Drunk defaults to subid0.
use super::*;
use crate::foundation::MovementLocalPosition;
fn source_direction(
    requested: StepDirection,
    active: bool,
    normal_draw: u8,
) -> (StepDirection, bool) {
    if !active || !(1..=25).contains(&normal_draw) {
        return (requested, false);
    }
    let direction = match normal_draw {
        1..=5 => StepDirection::North,
        6..=10 => StepDirection::West,
        11..=15 => StepDirection::South,
        16..=20 => StepDirection::East,
        _ => requested,
    };
    (direction, true)
}

pub(super) fn admitted_direction(
    runtime: &mut ChannelRuntimeV1,
    states: &super::super::actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    requested: StepDirection,
    now: u64,
) -> (StepDirection, bool) {
    use oteryn_gameplay_domain::domain::condition::StatusKind;
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    use sha2::{Digest, Sha256};
    if !states
        .get(runtime, actor, session)
        .is_some_and(|state| state.owned_conditions().has_status(StatusKind::Drunk, now))
    {
        return (requested, false);
    }
    // The retained Drome compatibility status has already been resolved by the
    // pure drome_step_direction owner with this same time and position. Every
    // other native Drunk condition keeps this ordinary source owner exactly once.
    if states.get(runtime, actor, session).is_some_and(|state| {
        state
            .owned_conditions()
            .active_at(
                oteryn_gameplay_domain::domain::condition::ConflictKey::Status(StatusKind::Drunk),
                now,
            )
            .is_some_and(|condition| condition.definition().key() == "drome.drunk")
    }) {
        return (requested, false);
    }
    // The connection routes only due, non-replayed walks here (serve/route.rs).
    // An owner ordinal distinguishes blocked attempts without a second movement store.
    let Ok(stamp) = runtime.issue_owner_work() else {
        return (requested, false);
    };
    let digest = Sha256::new()
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(stamp.generation().get().to_be_bytes())
        .chain_update(stamp.ordinal().get().to_be_bytes())
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let Ok(sample) = oteryn_simulation_determinism::deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(occurrence),
        "movement.drunk",
        0,
    ) else {
        return (requested, false);
    };
    let Ok(draw) =
        oteryn_gameplay_spell_rules::native_actor_states::normal_draw(1, 100, &mut |_, _| {
            i64::from(sample as u32)
        })
    else {
        return (requested, false);
    };
    source_direction(requested, true, draw as u8)
}

pub(super) fn speech(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    _runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    origin: Option<crate::foundation::MovementPositionSnapshot>,
) {
    let Some(map) = owner.bundle_map() else {
        return;
    };
    // The movement cue queue always owns canonical native coordinates. Its
    // egress projects that source frame to the selected wire frame once.
    let Some(position) = origin.map(|origin| origin.position()) else {
        return;
    };
    let captured_name =
        owner.states.chat.speaker_name(session).and_then(|name| {
            oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(&name).ok()
        });
    enqueue_speech(&map.facts, actor, captured_name, position);
}
fn enqueue_speech(
    facts: &crate::map::facts::BundleFacts,
    actor: ExactActorRef,
    captured_name: Option<oteryn_protocol_oteryn::entity_names::EntityDisplayName>,
    position: MovementLocalPosition,
) -> bool {
    let (Ok(x), Ok(y), Ok(floor)) = (
        position.x.try_into(),
        position.y.try_into(),
        position
            .floor
            .checked_neg()
            .ok_or(())
            .and_then(|floor| i8::try_from(floor).map_err(|_| ())),
    ) else {
        return false;
    };
    facts.emit_movement_speech(crate::map::facts::MovementSpeech {
        speaker: actor,
        position: crate::map::overlay::TilePos { x, y, floor },
        talk_type: 36,
        text: "Hicks!".into(),
        captured_name,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_drome_drunk_real_speech_queue_uses_canonical_native_floor()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (bytes, _) = crate::map::boot::tests::floor_bundle();
        let bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(|e| format!("{e:?}"))?;
        let facts = crate::map::facts::BundleFacts::palette(
            &bundle.manifest,
            crate::map::boot::tests::floor_items,
        )
        .map_err(|e| format!("{e:?}"))?;
        let id = |tag| {
            let mut bytes = [tag; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        };
        let world = crate::foundation::WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            crate::foundation::ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?,
            crate::foundation::NodeId::decode(&id(3)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            crate::foundation::ChannelContentPin::test(world),
        )
        .map_err(|e| format!("{e:?}"))?;
        let session = GameSessionId::decode(&id(4)).map_err(|e| format!("{e:?}"))?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(|e| format!("{e:?}"))?;
        let actor = runtime
            .commit_fresh_session(reservation)
            .map_err(|e| format!("{e:?}"))?;
        let name = oteryn_protocol_oteryn::entity_names::EntityDisplayName::new("Actual Player")
            .map_err(|e| format!("{e:?}"))?;
        for floor in [7, 8] {
            assert!(enqueue_speech(
                &facts,
                actor,
                Some(name),
                MovementLocalPosition {
                    x: 100,
                    y: 200,
                    floor
                }
            ));
            let rows = facts.movement_speech_since(0);
            let (_, line) = rows.last().ok_or("actual queued Drunk speech")?;
            assert_eq!(line.position.floor, -(floor as i8));
            assert_eq!((line.position.x, line.position.y), (100, 200));
            assert_eq!(line.speaker, actor);
            assert_eq!(line.captured_name, Some(name));
            assert_eq!(line.talk_type, 36);
            assert_eq!(line.text, "Hicks!");
        }
        Ok(())
    }
    #[test]
    fn srx3_drome_tanked_standard_drunk_uses_donor_walk_buckets_and_speech() {
        for (draw, direction) in [
            (1, StepDirection::North),
            (5, StepDirection::North),
            (6, StepDirection::West),
            (10, StepDirection::West),
            (11, StepDirection::South),
            (15, StepDirection::South),
            (16, StepDirection::East),
            (20, StepDirection::East),
        ] {
            assert_eq!(
                source_direction(StepDirection::North, true, draw),
                (direction, true)
            );
        }
        assert_eq!(
            source_direction(StepDirection::North, true, 21),
            (StepDirection::North, true)
        );
        assert_eq!(
            source_direction(StepDirection::North, true, 25),
            (StepDirection::North, true)
        );
        assert_eq!(
            source_direction(StepDirection::East, true, 26),
            (StepDirection::East, false)
        );
        assert_eq!(
            source_direction(StepDirection::East, false, 1),
            (StepDirection::East, false)
        );
    }
}
