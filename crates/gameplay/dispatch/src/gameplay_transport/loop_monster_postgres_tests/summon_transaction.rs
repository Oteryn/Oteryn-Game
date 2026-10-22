//! Actual one-pass summon registration/replay and Item transaction refusal.
use super::*;
use crate::ai_think::summon_plan::{MonsterThinkReader, summon_path::NativeMonsterThinkRead};
use crate::monster_summon::{NativeSummonCatalog, SummonError};
use crate::movement::{monster_map::CurrentMonsterMap, path_budget::ChannelPathBudget};

pub(super) async fn scenario(
    harness: &Harness,
    replay_only: bool,
    missing_child: bool,
) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = if missing_child {
        crate::content::native_gameplay::test_support::activated_loop_monster_missing_child(world)
    } else {
        crate::content::native_gameplay::test_support::activated_loop_monster(world, true)
    };
    let active = controller.active().ok_or("active generation")?;
    let (deps, rat) = dependencies(active, &pin)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells();
    if !replay_only && !missing_child {
        super::shared_budget::exercise(harness, &deps, &owner, rat).await?;
        // The budget test completed the walkback. Start the summon case from the pinned
        // admission cell again, retaining the same AI/schedule owner and its occurrence.
        crate::movement::relocate::relocate_creature(
            &mut *deps.runtime.lock().await,
            rat,
            deps.room.movement_cells(),
            &std::collections::BTreeSet::new(),
            cell(2),
        )
        .map_err(debug)?;
    }
    deps.initialize(&owner).await;
    owner.clock_origin -= std::time::Duration::from_secs(10);
    owner.drain_monster_melee().await;
    let first_report = owner
        .spell_states
        .lock()
        .await
        .monster_melee
        .creature_ai
        .last_report_for_test(rat)
        .ok_or("native report")?
        .clone();
    let children = deps.runtime.lock().await.native_summon_count(rat, None);
    assert_eq!(
        children,
        u32::from(!missing_child),
        "one native schedule creates a registered child or rolls back the refused registration; {first_report:?}"
    );
    assert_eq!(
        first_report.proposals.len(),
        1,
        "the same native think reports the admitted ranged Spell body"
    );
    assert_eq!(first_report.proposals[0].range_tiles, 7);
    assert_eq!(first_report.summon_proposals.len(), 1);
    check_scope_replay(
        harness,
        &deps,
        &owner,
        rat,
        &first_report,
        replay_only,
        missing_child,
        crate::content::native_gameplay::test_support::loop_monster_inner_digest(&pin),
    )
    .await?;
    if missing_child {
        assert_eq!(
            owner
                .spell_states
                .lock()
                .await
                .monster_melee
                .creature_ai
                .len(),
            1
        );
        assert_eq!(
            deps.runtime.lock().await.visible_entities().creatures.len(),
            1,
            "no unregistered physical child or native link survives rollback"
        );
    }
    if replay_only || missing_child {
        return Ok(());
    }
    let table_count = owner
        .spell_states
        .lock()
        .await
        .monster_melee
        .creature_ai
        .len();
    assert_eq!(
        table_count, 2,
        "the synchronous consumer registered the child"
    );
    assert_eq!(
        owner
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .last_report_for_test(rat)
            .ok_or("replayed report")?
            .sequence,
        first_report.sequence,
        "replaying the native occurrence did not run another think"
    );
    assert_eq!(deps.runtime.lock().await.native_summon_count(rat, None), 1);
    assert_eq!(
        owner
            .spell_states
            .lock()
            .await
            .monster_melee
            .creature_ai
            .len(),
        table_count
    );
    owner.clock_origin -= std::time::Duration::from_secs(1);
    owner.drain_monster_melee().await;
    assert_eq!(deps.runtime.lock().await.native_summon_count(rat, None), 2);
    owner.clock_origin -= std::time::Duration::from_secs(1);
    owner.drain_monster_melee().await;
    assert_eq!(
        deps.runtime.lock().await.native_summon_count(rat, None),
        2,
        "live cap includes registered children"
    );
    super::item_scope::assert_unknown_mint(harness, &deps, &owner).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn check_scope_replay(
    harness: &Harness,
    deps: &Dependencies,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    rat: ExactActorRef,
    report: &crate::ai_think::ThinkReport,
    retire: bool,
    missing_child: bool,
    inner_digest: [u8; 32],
) -> TestResult {
    use crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let proposal = report
        .summon_proposals
        .first()
        .ok_or("native summon proposal")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let mut runtime = deps.runtime.lock().await;
    let mut states = owner.spell_states.lock().await;
    let objects = deps.door.lock().await;
    let mut tx = harness.pool.begin().await?;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(
        runtime.binding().world_id(),
        runtime.binding().channel_id(),
    );
    let scope_authority = assert_spell_item_scope_in_transaction(
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
    let mut other_tx = harness.pool.begin().await?;
    let mut wrong = CurrentMonsterMap::new(
        &mut other_tx,
        &scope_authority,
        &runtime,
        &deps.room,
        &objects,
        native,
    );
    assert_eq!(
        wrong.read(&runtime, cell(2)).await,
        None,
        "another physical SQL transaction cannot borrow an old empty-tile result"
    );
    drop(wrong);
    other_tx.rollback().await?;
    let mut map = CurrentMonsterMap::new(
        &mut tx,
        &scope_authority,
        &runtime,
        &deps.room,
        &objects,
        native,
    );
    assert_eq!(map.read(&runtime, cell(2)).await, Some(true));
    let mut budget = ChannelPathBudget::default();
    let mut reader =
        NativeMonsterThinkRead::new(map, native, &mut budget, owner.owner_now().get(), false);
    reader
        .summon_facts(
            &runtime,
            proposal.occurrence,
            Some(proposal.target_with_path),
            None,
        )
        .await
        .ok_or("current path")?;
    let stamp = runtime.issue_owner_work().map_err(debug)?;
    let snapshot = runtime.companion_snapshot(rat).map_err(debug)?;
    let parent = crate::content::ProjectV2DefinitionRef {
        family: crate::content::ProjectV2Family::Creature,
        key: snapshot.state.policy.definition_key.clone(),
        revision: snapshot.state.policy.definition_revision.clone(),
    };
    let outer_catalog =
        NativeSummonCatalog::from_native(&runtime, native, &parent).map_err(debug)?;
    assert_ne!(inner_digest, native.source_digest());
    let inner_catalog = outer_catalog
        .requalified_for_test(inner_digest)
        .map_err(debug)?;
    let count = runtime.native_summon_count(rat, None);
    let table_count = states.monster_melee.creature_ai.len();
    let crate::gameplay_transport::actor_spell::ChannelSpellStates {
        monster_melee,
        monster_summons,
        ..
    } = &mut *states;
    assert_eq!(
        monster_summons.execute_registered(
            &mut runtime,
            stamp,
            &inner_catalog,
            proposal,
            &mut reader,
            native,
            &mut monster_melee.creature_ai,
            owner.owner_now().get()
        ),
        Err(SummonError::InvalidSource)
    );
    let replay = monster_summons.execute_registered(
        &mut runtime,
        stamp,
        &outer_catalog,
        proposal,
        &mut reader,
        native,
        &mut monster_melee.creature_ai,
        owner.owner_now().get(),
    );
    if missing_child {
        assert_eq!(replay, Err(SummonError::InvalidSource));
    } else {
        assert!(!replay.map_err(debug)?.newly_created);
    }
    assert_eq!(runtime.native_summon_count(rat, None), count);
    assert_eq!(monster_melee.creature_ai.len(), table_count);
    assert!(
        sqlx::query("SELECT 1/0")
            .execute(&mut **reader.map.transaction_for_test())
            .await
            .is_err()
    );
    assert!(
        !reader.map.validate_transaction(&runtime).await,
        "an aborted physical transaction cannot authorize cached empty tiles"
    );
    assert!(!reader.map.view().known(cell(2)));
    if retire {
        runtime.retire_owner_cycle();
        assert_eq!(
            monster_summons.execute_registered(
                &mut runtime,
                stamp,
                &outer_catalog,
                proposal,
                &mut reader,
                native,
                &mut monster_melee.creature_ai,
                owner.owner_now().get()
            ),
            Err(SummonError::StaleOwner)
        );
    }
    drop(reader);
    tx.rollback().await?;
    Ok(())
}
