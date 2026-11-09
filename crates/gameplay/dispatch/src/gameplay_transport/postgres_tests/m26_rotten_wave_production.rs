//! M26: actual production profile driver, current PG map and source native AI children.
//! A native qualification actor exercises the callback independently of opening
//! an unfinished Rotten Blood quest room; no room-qualification assertion is bypassed.
use super::TestResult;
use super::fixture::debug;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::spell_item_transaction::{
        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
    };
    const KEY: &str = "oteryn:creature.ichgahal";
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let room = owner
        .rooms()
        .gate_monster_ai()
        .ok_or("actual Instance AI room")?;
    assert!(
        room.source_world()
            .ok_or("source room")?
            .pending_mutable_placements()
            .is_empty()
    );
    let permit = owner.spell_lane_permit().await.ok_or("native wave lane")?;
    let mut runtime = owner.runtime.lock().await;
    assert!(
        runtime.encounter_slots().len() > 12,
        "the production activity factory reserves encounter slots beyond the five-player entry roster"
    );
    let mut states = owner.states.spell_states.lock().await;
    let mut attack = owner.states.attack.lock().await;
    let player = admitted.runtime_actor.ok_or("activity player")?;
    let position = runtime
        .read_actor_position(player)
        .map_err(debug)?
        .position();
    let now = owner.owner_now().get();
    let pass = owner.root.try_issue_semantic_pass().map_err(debug)?;
    let mut context = (owner, &mut *runtime, &mut *states, &permit);
    let boss = pass
        .run_with_context(&mut context, |holder, deadline, context| {
            Box::pin(async move {
                let (owner, runtime, states, permit) = context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let binding = runtime.binding();
                let authority = assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    binding.runtime_scope(),
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                let objects = owner.door.lock().await;
                let mut map = crate::movement::monster_map::CurrentMonsterMap::new(
                    &mut tx, &authority, runtime, room, &objects, native,
                );
                let mut selected = None;
                for (dx, dy) in [
                    (0, -2),
                    (1, -2),
                    (-1, -2),
                    (1, 0),
                    (-1, 0),
                    (0, -1),
                    (0, 1),
                    (1, -1),
                    (-1, -1),
                    (1, 1),
                    (-1, 1),
                ] {
                    let at = MovementLocalPosition {
                        x: position.x + dx,
                        y: position.y + dy,
                        floor: position.floor,
                    };
                    if map.read(runtime, at).await == Some(true)
                        && runtime.native_summon_cell_free(at)
                    {
                        // The callback's entire source placement envelope must
                        // be captured; an entry-edge cell is not that proof.
                        let mut captured = true;
                        for oy in -2..=2 {
                            for ox in -2..=2 {
                                captured &= map
                                    .read(
                                        runtime,
                                        MovementLocalPosition {
                                            x: at.x + ox,
                                            y: at.y + oy,
                                            floor: at.floor,
                                        },
                                    )
                                    .await
                                    .is_some();
                            }
                        }
                        if captured {
                            selected = Some(at);
                            break;
                        }
                    }
                }
                let at = selected.ok_or(crate::durability::DurabilityError::InvalidStoredState)?;
                // Test-only admission uses real source policies/health and the active
                // pin, under the independently proved current PG map transaction.
                let pin = runtime.content_pin().clone();
                let boss = runtime
                    .realize_native_qualification_spawn(pin, KEY, at)
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?
                    .actor;
                let record = native
                    .creature_profiles()
                    .records
                    .iter()
                    .find(|record| record.profile.target.key == KEY)
                    .ok_or(crate::durability::DurabilityError::InvalidStoredState)?;
                states
                    .monster_melee
                    .creature_ai
                    .admit_native_creature(runtime, native, boss, &record.profile.target, now)
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                assert!(
                    map.validate_transaction(runtime).await,
                    "actual source map transaction remains current"
                );
                drop(map);
                tx.rollback().await?;
                Ok(boss)
            })
        })
        .await
        .map_err(debug)?;
    let digest = native
        .source_digest()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let revisions = crate::ability::RevisionSet::new(
        "ruleset:ai-v1",
        &format!("content:{digest}"),
        "world:ai-v1",
        "formula:source-melee-baseline",
        "simulation:v1",
    )
    .map_err(debug)?;
    let mut first_wave = Vec::new();
    for tick in 0..=10 {
        let tick_now = now + tick * 1_000_000;
        // The actual visible-player wake path schedules each ambient think;
        // empty combat targets keep this case focused on the source callback.
        states.monster_melee.creature_ai.wake(
            position,
            |actor| {
                runtime
                    .read_actor_position(actor)
                    .ok()
                    .map(|row| row.position())
            },
            tick_now,
        );
        let prepared_names = owner.capture_native_health_names(&runtime).map_err(|_| "capture names")?;
        let read_names = owner.read_prepared_native_health_names(&permit, prepared_names).await;
        let names = owner.seal_native_health_names(&runtime, read_names).map_err(|_| "seal names")?;
        let (_, reported) = owner
            .drive_monster_profiles(
                &mut runtime,
                &mut states,
                &mut attack,
                room,
                native,
                &permit,
                &revisions,
                &[],
                tick_now,
                &names,
            )
            .await
            .map_err(|_| "production profile drive")?;
        assert!(
            reported.contains(&boss),
            "the real native scheduler commits this boss think"
        );
        let children = states.rotten_waves.wave_children(boss);
        if tick == 0 {
            assert!(
                !children.is_empty(),
                "committed native think physically creates its first source wave: {:?}",
                states.rotten_waves
            );
            first_wave = children.clone();
        }
        for child in children {
            assert_eq!(
                runtime
                    .companion_snapshot(child)
                    .map_err(debug)?
                    .state
                    .policy
                    .definition_key,
                "oteryn:creature.mushroom"
            );
            assert!(
                states.monster_melee.creature_ai.state(child).is_some(),
                "every successful source wave child enters native AI"
            );
            assert!(
                runtime.native_summon_master(child).is_none(),
                "source Game.createMonster remains masterless"
            );
        }
    }
    assert!(
        first_wave
            .iter()
            .all(|child| runtime.read_actor_position(*child).is_err()),
        "source cadence removes the exact previous wave"
    );
    assert!(
        first_wave
            .iter()
            .all(|child| states.monster_melee.creature_ai.state(*child).is_none()),
        "replacement also retires the previous AI registrations"
    );
    assert!(
        !states.rotten_waves.wave_children(boss).is_empty(),
        "the replacement wave is runnable"
    );
    eprintln!(
        "M26_ROTTEN_WAVE production_driver=true pg_map=true native_ai=true cadence_cleanup=true"
    );
    Ok(())
}
