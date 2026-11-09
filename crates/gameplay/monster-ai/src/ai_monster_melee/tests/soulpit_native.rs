//! Actual pinned native melee closure and real Source variant carrier -> Ability bite.
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use oteryn_gameplay_domain::domain::soulpit::{BossAbility, SoulpitStack, wave_stacks};
use oteryn_gameplay_entities::carrier::soulpit::SoulpitWaveRequest;

#[test]
fn srx3_soulpit_actual_native_bite_scales_before_target_block_and_replays() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "native Soulpit bite thread panicked")?
}
fn err(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
fn scenario() -> Result<(), String> {
    let world = WorldId::decode(&uuid(1)).map_err(err)?;
    let (controller, baseline) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active native source")?;
    let native = active.native_gameplay().ok_or("native source bodies")?;
    let room = active
        .qualify_activity_source_room(baseline.qualified_room(), PinnedActivitySourceRoom::Soulpit)
        .map_err(err)?;
    let source_world = room.source_world().ok_or("actual Soulpit map")?;
    let known = |at: MovementLocalPosition| {
        source_world.lookup(
            room.movement_cells().scope(),
            crate::content::LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) == Ok(crate::content::CollisionClass::Walkable)
    };
    let spawn = MovementLocalPosition {
        x: 32376,
        y: 31144,
        floor: -8,
    };
    if !known(spawn) {
        return Err("actual source boss cell is not walkable".into());
    }
    let target_position = [(0, -1), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .map(|(dx, dy)| MovementLocalPosition {
            x: spawn.x + dx,
            y: spawn.y + dy,
            ..spawn
        })
        .find(|at| known(*at))
        .ok_or("actual adjacent player cell")?;
    let mut pool = Vec::new();
    for y in 31132..=31153 {
        for x in 32362..=32390 {
            let at = MovementLocalPosition { x, y, floor: -8 };
            if at != spawn && at != target_position && known(at) {
                pool.push(at);
            }
        }
    }
    if pool.len() < 6 {
        return Err("actual source room has fewer than six extra cells".into());
    }
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == "oteryn:creature.oppressor")
        .ok_or("unchanged native Oppressor")?;
    let closure = record
        .attack_definitions
        .as_ref()
        .ok_or("actual source attack closure")?;
    let source = MeleeSource::from_native(
        &record.profile.target,
        0,
        &closure.records,
        &closure.profiles,
        native.source_digest(),
    )
    .map_err(err)?;
    let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) =
        &record.behavior.as_ref().ok_or("source behavior")?.data
    else {
        return Err("actual behavior type".into());
    };
    let schedule = behavior
        .attacks
        .first()
        .ok_or("actual first melee schedule")?;
    let definition = MeleeDefinition::new_native(
        native.source_digest(),
        schedule.interval_ms,
        schedule.chance_ppm,
        source.minimum,
        source.maximum,
    )
    .ok_or("native definition")?
    .with_entry_index(0);
    for (stack, wave, ability, roll, expected) in [
        (SoulpitStack::Normal, 1, BossAbility::Enrage, 100, 27),
        (SoulpitStack::Strong, 2, BossAbility::Enrage, 100, 35),
        (SoulpitStack::Powerful, 3, BossAbility::Enrage, 100, 55),
        (SoulpitStack::Boss, 4, BossAbility::Enrage, 100, 105),
        (SoulpitStack::Boss, 4, BossAbility::Overpower, 1, 157),
    ] {
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            native.source_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (spawn.x, spawn.y, spawn.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            crate::foundation::RuntimeScopeRefV1::instance(world, uuid(2)).map_err(err)?,
            NodeId::decode(&uuid(3)).map_err(err)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin,
        )
        .map_err(err)?;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(err)?;
        let session = GameSessionId::decode(&uuid(4)).map_err(err)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(err)?;
        let target = runtime.commit_fresh_session(reservation).map_err(err)?;
        runtime
            .initialize_pinned_test_position(target, target_position)
            .map_err(err)?;
        let selected = wave_stacks(wave)
            .map_err(err)?
            .iter()
            .position(|value| *value == stack)
            .ok_or("actual wave stack")?;
        let mut positions = [spawn; 7];
        let mut next = 0;
        for (ordinal, position) in positions.iter_mut().enumerate() {
            if ordinal != selected {
                *position = *pool.get(next).ok_or("actual extra spawn cell")?;
                next += 1;
            }
        }
        let actors = {
            let (carrier, continuity, pin) = runtime.soulpit_kernel_parts().map_err(err)?;
            carrier
                .spawn_soulpit_wave(
                    continuity,
                    pin,
                    native.policies(),
                    SoulpitWaveRequest {
                        handoff_id: uuid(7),
                        wave,
                        species: record.profile.target.key.clone(),
                        positions: positions.map(Some),
                        boss_ability: ability,
                        now_micros: 0,
                    },
                    known,
                )
                .map_err(err)?
        };
        let issuer = actors[selected]
            .ok_or("actual Source variant admission")?
            .actor;
        let intent = AiAbilityAdapter::bite(issuer, 1, target, session, 0, 0);
        let definition = CreatureBiteDefinition::from_native_source_with_roll(
            &source, &runtime, intent, definition, 20, roll,
        )
        .ok_or("actual raw20 source qualification")?;
        let mut ledger = CreatureBiteLedger::default();
        let mut vitals = Vitals {
            health: 1000,
            revision: 0,
            armor: 10,
            defence: None,
            protection: None,
        };
        let applied = commit_ai_bite(
            &mut ledger,
            &mut runtime,
            &mut vitals,
            intent,
            definition,
            revisions()?,
            ReentryProtection {
                protected_until: None,
            },
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .map_err(err)?;
        assert_eq!(
            applied.requested, expected,
            "actual native Source stack{stack:?}"
        );
        assert_eq!(
            applied.damage.applied,
            expected - 5,
            "armor10/draw0 blocks5 after Source scaling"
        );
        let replay = commit_ai_bite(
            &mut ledger,
            &mut runtime,
            &mut vitals,
            intent,
            definition,
            revisions()?,
            ReentryProtection {
                protected_until: None,
            },
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .map_err(err)?;
        assert_eq!(replay, applied);
        assert_eq!(
            vitals.health,
            1000 - (expected - 5),
            "retry must not hurt the player twice"
        );
    }
    Ok(())
}
