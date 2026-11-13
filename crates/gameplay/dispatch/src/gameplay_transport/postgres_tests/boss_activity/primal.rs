//! Primal's existing endgame assertions through its actual pinned source entry.
use super::super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;

pub(super) async fn scenario(admin: String) -> TestResult {
    reset();
    let (scene, activation_pin) = SourceScene::new_with_activation(
        PinnedActivitySourceRoom::Primal,
        MovementLocalPosition {
            x: 33548,
            y: 32752,
            floor: 14,
        },
    )?;
    let deps = scene.dependencies()?;
    let h = Harness::create_with_admission_seed(admin, "native_primal_entry", false, false).await?;
    let evidence = super::super::house_transfer::RecoverySource::start(&h).await?;
    super::super::house_transfer::prepare_owned_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &evidence.evidence;
    // Keep the actual source origin, but deliberately withhold its registration
    // for the original unavailable-source counterfactual below.
    owner.active_generation = scene.controller.active();
    owner.rooms = SeamRooms::new(Some(&scene.room), Some(&scene.room));
    let map_revision = crate::map::overlay::map_revision(
        owner
            .bundle_map()
            .ok_or("actual source bundle map")?
            .overlay
            .base(),
    );
    super::issue_content_and_reset_with_runtime_pin(&h, &activation_pin, &scene.pin, &map_revision)
        .await?;
    let mut admitted = deps.admitted;
    let (offered, scope, generation) = {
        let runtime = owner.runtime.lock().await;
        (
            crate::gameplay_transport::world_required_capabilities::with_native_map_presentation_offer(
                BUNDLE_WORLD_OFFERED_CAPABILITIES, owner.active_generation,
                owner.effective_rooms().gate_wire_position(), runtime.content_pin(),
            ),
            runtime.binding().runtime_scope(),
            runtime.binding().scope_generation().get(),
        )
    };
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[11,18,30], scope, generation, "policy-1", offered,
    ).map_err(debug)?;
    let supported = [
        4,
        6,
        7,
        10,
        11,
        15,
        18,
        23,
        30,
        92,
        wire::CAPABILITY_BOSS_ENCOUNTER_V1,
        385,
        oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_NATIVE_OBJECT_APPEARANCE_V1,
    ];
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::connection::fresh_capability_selection(&owner, &supported)
            .await
            .ok_or("actual Primal source capability selection")?;
    owner
        .required_capabilities
        .authenticate_resume_support(&supported, admitted.continuity.selected_capabilities)
        .map_err(debug)?;
    // A real level499 Character must fail the literal level500 source policy.
    // No vitals/HP, source policy or gate is overridden by this fixture.
    prepare_player(&h, &owner, &mut admitted, 499).await?;
    super::super::item_look::seed_main_backpack(&h, &[]).await?;
    let instance = super::super::activity_runtime::allocate_ready_instance(&h, 88).await?;
    let assignment = h
        .root
        .read_runtime_scope_assignment(instance)
        .await
        .map_err(debug)?
        .ok_or("allocated Instance")?;
    let source_cell = MovementLocalPosition {
        x: 33548,
        y: 32752,
        floor: 14,
    };
    let (_, refused, barrier) = super::exchange(
        &owner,
        &mut admitted,
        "oteryn:creature.the_primal_menace",
        false,
    )
    .await?;
    assert_eq!(
        refused,
        BossActivityReply::Refused(BossActivityRefusal::ContentUnavailable)
    );
    assert!(barrier.is_none());
    assert_unentered(&h, &owner, admitted, source_cell).await?;
    install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Primal)?;
    let (_, refused, barrier) = super::exchange(
        &owner,
        &mut admitted,
        crate::gameplay_transport::primal_entry::BOSS,
        false,
    )
    .await?;
    assert_eq!(
        refused,
        BossActivityReply::Refused(BossActivityRefusal::Level)
    );
    assert!(barrier.is_none());
    assert_unentered(&h, &owner, admitted, source_cell).await?;
    let definition = crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone())
        .read_boss_room(
            &authority,
            &h.node,
            admitted.item_fence.ok_or("current source fence")?,
            crate::gameplay_transport::primal_entry::BOSS,
        )
        .await?
        .ok_or("authored source catalogue is visible to the current fence")?;
    let allocations =
        crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone())
            .ready_allocations(&authority, &h.node, admitted.world_id, None)
            .await?;
    assert!(
        allocations
            .iter()
            .any(|allocation| allocation.scope == instance
                && allocation.generation.get() == assignment.ownership_generation
                && allocation.source_revision == assignment.source_revision),
        "allocated Instance remains independently ready: {allocations:?}"
    );
    let registered = owner
        .activity_runtimes
        .get(instance, assignment.ownership_generation)
        .ok_or("registered actual Primal source Instance")?;
    assert!(
        crate::gameplay_transport::native_activity::boss_room_native_ready(
            &*registered.runtime.lock().await,
            &definition,
        ),
        "the actual source room contains its exact living boss before entry"
    );
    // Preserve the existing receipt-backed level transition, then use the
    // canonical source key rather than renaming a generic catalogue row.
    super::endgame::prepare_level(&owner, admitted, 500).await?;
    assert_unentered(&h, &owner, admitted, source_cell).await?;
    let fence = admitted.item_fence.ok_or("current source fence")?;
    let before_state = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let state = states
            .get(&runtime, deps.actor, admitted.game_session_id)
            .ok_or("source live state")?;
        (state.revision(), state.vitals(), state.saved_vitals())
    };
    let old = admitted;
    let boss_key = crate::gameplay_transport::primal_entry::BOSS;
    let (command, reply, barrier) = super::exchange(&owner, &mut admitted, boss_key, true).await?;
    let BossActivityReply::Accepted {
        handoff,
        deadline_unix_ms: Some(_),
    } = reply
    else {
        return Err(format!("expected real entry: {reply:?}").into());
    };
    let barrier = barrier.ok_or("scope barrier missing")?;
    assert_eq!(barrier.handoff, handoff);
    assert_eq!(barrier.source_session, *old.game_session_id.as_bytes());
    assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
    assert_ne!(old.game_session_id, admitted.game_session_id);
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = instance
    else {
        return Err("Instance expected".into());
    };
    assert_eq!(
        barrier.scope,
        ActivityScope::Instance {
            world: *world_id.as_bytes(),
            instance: instance_id
        }
    );
    assert_eq!(barrier.connection_generation, 1);
    assert_eq!(barrier.scope_generation, assignment.ownership_generation);
    assert_eq!(
        barrier.lease_generation,
        fence.character_lease_generation + 1
    );
    assert_eq!(
        barrier.selection.as_slice(),
        old.continuity.selected_capabilities.as_slice()
    );
    assert_ne!(
        admitted
            .controller
            .ok_or("destination controller")?
            .transport,
        old.controller.ok_or("source controller")?.transport
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(deps.actor, old.game_session_id)
            .is_err()
    );
    assert_eq!(
        admitted
            .item_fence
            .ok_or("actual destination fence")?
            .runtime_scope,
        instance
    );
    let destination = owner
        .resolve_runtime_view(admitted)
        .await
        .map_err(|_| "resolve Instance owner")?
        .ok_or("owner actually changed")?;
    let next_actor = admitted.runtime_actor.ok_or("destination actor")?;
    assert!(
        destination
            .refresh_pvp_locks(next_actor, admitted.game_session_id)
            .await,
        "activity Instance retains the ordinary PvP reader, without House custody"
    );
    {
        let runtime = destination.runtime.lock().await;
        assert_eq!(runtime.binding().runtime_scope(), instance);
        assert_eq!(
            runtime
                .read_actor_position(next_actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 33565,
                y: 32758,
                floor: 15
            }
        );
        let states = destination.states.spell_states.lock().await;
        let state = states
            .get(&runtime, next_actor, admitted.game_session_id)
            .ok_or("moved live state")?;
        assert_eq!(
            (state.revision(), state.vitals(), state.saved_vitals()),
            before_state
        );
    }
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert_eq!(
        store
            .current_session_at(old.game_session_id)
            .await?
            .0
            .session_state(),
        crate::foundation::GameSessionState::Terminal
    );
    assert_eq!(
        item_fence_of(store.current_session_at(admitted.game_session_id).await?.0),
        admitted.item_fence
    );
    let replay = owner
        .retry_native_activity(old, command, boss_key)
        .await
        .map_err(|_| "same attempt replay")?
        .ok_or("retained attempt")?;
    let crate::gameplay_transport::native_activity::NativeActivityOutcome::Committed(replayed) =
        replay
    else {
        return Err("replay refused".into());
    };
    assert_eq!(replayed.handoff_id, handoff);
    owner
        .publish_activity_commit(&replayed)
        .map_err(|_| "idempotent publication")?;
    assert!(
        owner
            .activity_runtimes
            .pending_switch(old)
            .map_err(|_| "pending read")?
            .is_none(),
        "replay must not resurrect a delivered barrier"
    );
    let counts: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_activity_handoff_groups),(SELECT count(*) FROM game_boss_room_activities)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1));
    let mut forged = admitted;
    forged
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .scope_ownership_generation =
        crate::foundation::ScopeOwnershipGeneration::new(assignment.ownership_generation + 1)
            .map_err(debug)?;
    assert!(owner.resolve_runtime_view(forged).await.is_err());
    let policy = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native Primal")?
        .policies()
        .policies()
        .iter()
        .find(|policy| policy.definition_key == "oteryn:creature.the_primal_menace")
        .ok_or("native Primal policy")?;
    // Reuse every existing death/XP/hazard/53-byte replay/corpse/personal-reward
    // assertion verbatim in its original owner, against this actual source Instance.
    super::endgame::settle(&h, &destination, admitted, policy, Case::PrimalDefeat).await?;
    eprintln!("L3_ENDGAME_QUEST_ENTRY POSTGRES_CASE_COMPLETED oteryn:creature.the_primal_menace");
    eprintln!("BOSS_ACTIVITY_NATIVE_ENTRY POSTGRES_CASE_COMPLETED");
    eprintln!("BOSS_ACTIVITY_SCOPE_BARRIER POSTGRES_CASE_COMPLETED");
    eprintln!("BOSS_ACTIVITY_STATE_AND_REPLAY POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn assert_unentered(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: MovementLocalPosition,
) -> TestResult {
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_handoff_groups")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_boss_cooldowns")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("source actor")?)
            .map_err(debug)?
            .position(),
        source
    );
    Ok(())
}
