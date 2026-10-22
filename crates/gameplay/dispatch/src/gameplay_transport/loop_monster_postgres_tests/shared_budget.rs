//! Real PG Map reads with the existing Channel path ledger and retained request owner.
use super::*;
use crate::ai_think::summon_plan::summon_path::NativeMonsterThinkRead;
use crate::movement::monster_map::CurrentMonsterMap;

pub(super) async fn exercise(
    harness: &Harness,
    deps: &Dependencies,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    rat: ExactActorRef,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let mut runtime = deps.runtime.lock().await;
    let mut states = owner.spell_states.lock().await;
    let mut attack = owner.attack.lock().await;
    let objects = deps.door.lock().await;
    let mut tx = harness.pool.begin().await?;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(
        runtime.binding().world_id(),
        runtime.binding().channel_id(),
    );
    let authority =
        crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            scope,
            runtime.binding().scope_generation().get(),
        )
        .await
        .map_err(debug)?;
    let map = CurrentMonsterMap::new(&mut tx, &authority, &runtime, &deps.room, &objects, native);
    let root =
        oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(native.source_digest());
    let revisions =
        crate::ability::RevisionSet::new("rules-1", "content-1", "world-1", "formula-1", "sim-1")
            .map_err(debug)?;
    let policy = runtime.companion_snapshot(rat).map_err(debug)?.state.policy;
    let creature = crate::content::ProjectV2DefinitionRef {
        family: crate::content::ProjectV2Family::Creature,
        key: policy.definition_key.clone(),
        revision: policy.definition_revision.clone(),
    };
    let crate::gameplay_transport::actor_spell::ChannelSpellStates {
        monster_melee,
        monster_summons,
        ..
    } = &mut *states;
    let table = &mut monster_melee.creature_ai;
    table
        .admit_native_creature(&runtime, native, rat, &creature, 0)
        .map_err(debug)?;
    let mut reader = NativeMonsterThinkRead::new(map, native, &mut attack.path_budget, 0, false);
    // No PlayerSpellState is installed yet; the real selected-target fact is no target.
    table
        .run_due_current_thinks(
            &mut runtime,
            0,
            &root,
            &revisions,
            &[],
            &mut reader,
            |runtime, table, actor, report, reader| {
                monster_summons.consume_report(runtime, table, native, actor, report, reader, 0)
            },
        )
        .await;
    assert!(reader.budget.admits(0, false, false));
    while reader.budget.admits(0, false, false) {
        reader
            .budget
            .charge(false, crate::movement::path_search::PATH_SEARCH_WORK);
    }
    let next = table
        .drive_current_movement(
            &mut runtime,
            deps.room.movement_cells(),
            objects.collision_cells(),
            native,
            &root,
            0,
            reader.budget,
            false,
            &mut reader.map,
        )
        .await;
    assert_eq!(next, Some(50_000));
    assert_eq!(table.step_due_for_test(rat), None);
    assert_eq!(
        runtime.read_actor_position(rat).map_err(debug)?.position(),
        cell(2)
    );
    let sequence = table.last_report_for_test(rat).ok_or("report")?.sequence;
    table
        .drive_current_movement(
            &mut runtime,
            deps.room.movement_cells(),
            objects.collision_cells(),
            native,
            &root,
            50_000,
            reader.budget,
            false,
            &mut reader.map,
        )
        .await;
    let due = table
        .step_due_for_test(rat)
        .ok_or("retained search scheduled its step")?;
    table
        .drive_current_movement(
            &mut runtime,
            deps.room.movement_cells(),
            objects.collision_cells(),
            native,
            &root,
            due,
            reader.budget,
            false,
            &mut reader.map,
        )
        .await;
    assert_eq!(
        runtime.read_actor_position(rat).map_err(debug)?.position(),
        cell(3)
    );
    assert_eq!(
        table.last_report_for_test(rat).ok_or("report")?.sequence,
        sequence,
        "shared-budget retry moved without another think"
    );
    let home_due = table.step_due_for_test(rat).ok_or("second native step")?;
    table
        .drive_current_movement(
            &mut runtime,
            deps.room.movement_cells(),
            objects.collision_cells(),
            native,
            &root,
            home_due,
            reader.budget,
            false,
            &mut reader.map,
        )
        .await;
    assert_eq!(
        runtime.read_actor_position(rat).map_err(debug)?.position(),
        cell(4)
    );
    assert_eq!(
        table.due(rat),
        None,
        "the completed walkback idles until the real player admission wakes it"
    );
    assert_eq!(
        table.last_report_for_test(rat).ok_or("report")?.sequence,
        sequence
    );
    drop(reader);
    tx.rollback().await?;
    Ok(())
}
