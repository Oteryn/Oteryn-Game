//! Actual source handoff proves a static Instance needs no Channel Item initializer.
use super::fixture::{Harness, debug, id};
use super::srx3_activity_source_fixture::{SourceScene, install_scene};
use super::{Dependencies, TestResult};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::boss_encounter as wire;
use std::time::Instant;
const SPITE_KEY: &str = "oteryn:creature.goshnar_s_spite";

#[test]
fn m26_spite_postgres_static_instance_map_needs_no_channel_item_custody() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| scenario(admin).await)
}

async fn scenario(admin: String) -> TestResult {
    let started = Instant::now();
    super::reset();
    let h = Harness::create_with_admission_seed(admin, "m26_static_instance", true, false).await?;
    let evidence = super::house_transfer::RecoverySource::start(&h).await?;
    super::house_transfer::prepare_owned_session(&h).await?;
    let source = PinnedActivitySourceRoom::GoshnarsSpite;
    // Crystal soul_war.lua:296: the first actual Spite waiting pad.
    let scene = SourceScene::new(
        source,
        MovementLocalPosition {
            x: 33774,
            y: 31634,
            floor: 14,
        },
    )?;
    let active = scene
        .controller
        .active()
        .ok_or("active production content")?;
    let native = active
        .native_gameplay()
        .ok_or("native production content")?;
    let template = active
        .qualify_soul_war_activity_template(&scene.room, "content-1", source)
        .map_err(debug)?;
    assert_eq!(
        (
            template.minimum_level(),
            template.minimum_players(),
            template.after_kill_seconds()
        ),
        (250, 1, 60)
    );
    let lever = *template.lever_positions().first().ok_or("source lever")?;
    let deps = Dependencies::with_content_and_capacity(
        scene.pin.clone(),
        scene.room.clone(),
        native.spell_book().clone(),
        64,
    )?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.evidence = &evidence.evidence;
    install_scene(&mut owner, &scene, source)?;
    // A qualified level-250 participant, with the actual native progression policy.
    let progression = native
        .progression()
        .ok_or("progression")?
        .policy_for("profile-1", "ruleset-1", "content-1")
        .map_err(debug)?;
    let xp = progression
        .thresholds
        .iter()
        .find(|row| row.level == 250)
        .ok_or("level 250 threshold")?
        .minimum_experience
        .get();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=250,total_experience=$2,simulation_revision=$3,evidence_revision=$4,declaration_revision=$5,policy_revision=$6,reward_revision=$7 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(xp).bind(&progression.context.simulation).bind(&progression.context.evidence)
        .bind(&progression.context.declaration).bind(&progression.policy_revision).bind(&progression.reward_revision).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0,
    );
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[wire::CAPABILITY_BOSS_ENCOUNTER_V1],
    )
    .ok_or("boss capability")?;
    let current = crate::gameplay_transport::character_gameplay_fence::current(
        &owner,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("current source task fence")?;
    let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
    let grant = owner
        .root
        .commit_soul_war_command(
            owner.character,
            owner.holder,
            current,
            CommandRef::new(admitted.game_session_id, CommandId::new(1).map_err(debug)?),
            crate::durability::quest_state::soul_war::SoulWarCommand::Start,
        )
        .await?;
    assert_eq!(grant.result, Ok(()));
    // This fenced owner call prepares the persisted task precondition; it did
    // not consume a transport envelope. Keep the real connection's fresh
    // command-ingress position, which the production resume gate verifies.
    slot.observe_endgame_progression();
    drop(slot);
    admitted.item_fence = item_fence_of(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0,
    );
    let fence = admitted.item_fence.ok_or("origin fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: lever.x,
                y: lever.y,
                floor: i16::try_from(lever.z)?
            }
        );
        owner
            .states
            .spell_states
            .try_lock()
            .map_err(debug)?
            .initialize(
                &runtime,
                deps.actor,
                admitted.game_session_id,
                crate::spell::cast::CharacterCastFacts {
                    level: 250,
                    max_health: 1_000_000,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("qualified player state")?;
    }
    owner.admit_progression_session(&mut admitted).await;
    assert!(
        owner
            .progression_binding(admitted.game_session_id)
            .is_some(),
        "production progression admission"
    );
    // Give the existing classless fixture its actual fenced native class choice.
    // Keep the qualified level-250 progression and source room unchanged.
    {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "class choice fence")?
            .ok_or("class choice fence")?;
        let before = owner
            .root
            .read_character_build_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
            "content-1",
        )
        .map_err(debug)?;
        let after =
            crate::durability::character_build::convert_vocation(&formula, &before, "knight")
                .map_err(debug)?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        slot.commit_build(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::character_build::BuildChangeRequest {
                occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(
                    182,
                ))
                .map_err(debug)?,
                cause: crate::durability::character_build::BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            &formula,
            None,
        )
        .await
        .map_err(debug)?;
    }
    super::item_look::seed_main_backpack(&h, &[]).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    owner
        .register_current_runtime()
        .await
        .map_err(|_| "register origin")?;
    owner
        .activity_runtimes
        .remember(admitted)
        .map_err(|_| "remember origin")?;
    super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
    let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
    let origin_session = admitted.game_session_id;
    let (_, reply, barrier) = super::boss_activity::exchange_command(
        &owner,
        &mut admitted,
        template.boss_key(),
        true,
        wire::COMMAND_TYPE_BOSS_LEVER,
    )
    .await?;
    assert!(matches!(reply, wire::BossActivityReply::Accepted { .. }));
    assert!(barrier.is_some());
    assert_eq!(
        admitted.item_fence.ok_or("Instance fence")?.runtime_scope,
        instance
    );
    assert_ne!(admitted.game_session_id, origin_session);
    let destination = owner
        .resolve_runtime_view(admitted)
        .await
        .map_err(|_| "resolve actual destination")?
        .ok_or("destination owner")?;
    assert!(
        destination
            .progression_binding(admitted.game_session_id)
            .is_some(),
        "progression binding follows the real handoff"
    );
    eprintln!("M26_SPITE_STAGE source_entry_committed");
    admitted.runtime_actor.ok_or("Instance player")?;
    let _boss = {
        let runtime = destination.runtime.lock().await;
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find(|(target, _, _)| {
                runtime
                    .companion_snapshot(*target)
                    .is_ok_and(|row| row.state.policy.definition_key == SPITE_KEY)
            })
            .map(|row| row.0)
            .ok_or("source boss")?
    };
    {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        use crate::map::boot::BundleCollisionIndexExt;
        use crate::spell::source_map_initialization::CurrentNativeMapInitialization;
        let runtime = destination.runtime.lock().await;
        let room = destination
            .rooms()
            .gate_field_step_ingress()
            .ok_or("actual bounded source Instance map")?;
        assert!(
            destination
                .rooms()
                .gate_source_map_initialization()
                .is_none(),
            "the actual Bundle keeps the SourceWorld Item initializer closed"
        );
        assert_eq!(room.pinned_activity_source_room(), Some(source));
        let NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index() else {
            return Err("actual Instance Bundle".into());
        };
        let NativeMovementCollisionIndex::Bundle(origin) = deps.room.movement_cells().index()
        else {
            return Err("actual origin Bundle".into());
        };
        assert_eq!(index.map().overlay.runtime_scope(), instance);
        assert_eq!(
            origin.map().overlay.runtime_scope(),
            super::fixture::fence(1)?.runtime_scope
        );
        assert_ne!(
            index.map().overlay.runtime_scope(),
            origin.map().overlay.runtime_scope(),
            "the Instance owns its map instead of borrowing the origin Channel overlay"
        );
        assert!(runtime.owner_fence().is_ok());
        assert_eq!(runtime.binding().runtime_scope(), instance);
        assert_eq!(
            runtime.binding().scope_generation(),
            admitted
                .item_fence
                .ok_or("Instance Item fence")?
                .scope_ownership_generation
        );
        let pin = runtime.content_pin();
        assert_eq!(
            (
                pin.world_id(),
                pin.activation_sequence(),
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                pin.map_revision_digest(),
            ),
            (
                instance.world_id(),
                active.activation_sequence(),
                active.identity().server_artifact_digest(),
                active.identity().client_artifact_digest(),
                room.frame_binding().digest(),
                room.map_revision_digest(),
            )
        );
        assert_eq!(
            room.compiled().server_digest(),
            pin.server_artifact_digest()
        );
        assert_eq!(
            room.compiled().client_digest(),
            pin.client_artifact_digest()
        );
        assert_eq!(native.source_digest(), pin.server_artifact_digest());
        assert_eq!(room.movement_cells().scope().world_id, instance.world_id());
        assert_eq!(
            room.movement_cells().scope().generation_digest,
            pin.server_artifact_digest()
        );
        assert!(matches!(
            CurrentNativeMapInitialization::qualify_static_instance(&runtime, room, native),
            Err("source map initializer requires actual source world")
        ));
        assert!(matches!(
            CurrentNativeMapInitialization::qualify_current_binding(&runtime, room, native),
            Err("source map initializer requires actual source world")
        ));
        assert!(
            CurrentNativeMapInitialization::qualify_static_instance(&runtime, &deps.room, native)
                .is_err(),
            "the origin Bundle cannot grant a SourceWorld empty-placement proof"
        );
    }
    let custody_counts = "SELECT (SELECT count(*) FROM game_native_map_item_receipts), (SELECT count(*) FROM game_native_map_item_audit)";
    let custody_before: (i64, i64) = sqlx::query_as(custody_counts).fetch_one(&h.pool).await?;
    for _ in 0..2 {
        assert!(
            destination.ensure_source_map_initialized().await,
            "the real scoped Bundle is ready on initial and repeated Movement preflight"
        );
        assert!(
            destination
                .states
                .spell_states
                .lock()
                .await
                .source_map_initialized
                .is_none(),
            "Bundle readiness never claims Channel Item initialization"
        );
    }
    let custody_after: (i64, i64) = sqlx::query_as(custody_counts).fetch_one(&h.pool).await?;
    assert_eq!(
        custody_after, custody_before,
        "initial and repeated Bundle readiness writes no Channel map Item receipts or audit"
    );
    let gameplay = crate::gameplay_transport::character_gameplay_fence::current(
        &destination,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("activity lifecycle fence")?;
    let facts = read_activity_lifecycle(&destination, &gameplay, native.source_digest()).await?;
    assert!(
        facts.is_ok(),
        "qualified activity retains its actual lifecycle facts: {facts:?}"
    );
    let facts = facts.map_err(debug)?;
    assert_eq!(facts.level, 250);
    assert_eq!(facts.fence, gameplay.item_fence());
    assert_eq!(facts.equipment.content_digest, native.source_digest());
    let mut stale = gameplay;
    stale.expected_character_revision =
        crate::domain::CharacterRevision::new(gameplay.expected_character_revision.get() + 1)
            .map_err(debug)?;
    let refused = read_activity_lifecycle(&destination, &stale, native.source_digest()).await?;
    assert!(
        matches!(
            refused,
            Err(
                crate::durability::character_equipment::EquipmentError::Rejected(
                    "activity lifecycle Character revision"
                )
            )
        ),
        "stale lifecycle revision is refused concretely: {refused:?}"
    );
    let refused = read_activity_lifecycle(&destination, &gameplay, [0xb6; 32]).await?;
    assert!(
        matches!(
            refused,
            Err(
                crate::durability::character_equipment::EquipmentError::Rejected(
                    "activity equipment destination Content pin"
                )
            )
        ),
        "wrong lifecycle content is refused concretely: {refused:?}"
    );
    let command = CommandRef::new(admitted.game_session_id, CommandId::new(7).map_err(debug)?);
    let cast = read_activity_cast(
        &destination,
        &gameplay.item_fence(),
        command,
        native.source_digest(),
    )
    .await?;
    assert!(
        cast.is_ok(),
        "activity familiar cast reads its actual bound facts: {cast:?}"
    );
    let cast = cast.map_err(debug)?;
    assert_eq!(cast.command, command);
    assert_eq!(cast.fence, gameplay.item_fence());
    assert_eq!(cast.level, 250);
    assert_eq!(cast.equipment.content_digest, native.source_digest());
    let foreign_command = CommandRef::new(
        deps.admitted.game_session_id,
        CommandId::new(7).map_err(debug)?,
    );
    assert_ne!(foreign_command.game_session_id(), command.game_session_id());
    let refused = read_activity_cast(
        &destination,
        &gameplay.item_fence(),
        foreign_command,
        native.source_digest(),
    )
    .await?;
    assert!(
        matches!(
            refused,
            Err(
                crate::durability::character_equipment::EquipmentError::Rejected(
                    "activity cast command session"
                )
            )
        ),
        "another session's command grants no activity cast facts: {refused:?}"
    );
    let refused =
        read_activity_cast(&destination, &gameplay.item_fence(), command, [0xb6; 32]).await?;
    assert!(
        matches!(
            refused,
            Err(
                crate::durability::character_equipment::EquipmentError::Rejected(
                    "activity equipment destination Content pin"
                )
            )
        ),
        "a command cannot substitute another activity content generation: {refused:?}"
    );
    let owned = read_owned_activity_cast(&destination, &gameplay.item_fence(), command).await?;
    assert!(
        owned.is_ok(),
        "ordinary activity cast reads actual Character and Wheel facts: {owned:?}"
    );
    let refused =
        read_owned_activity_cast(&destination, &gameplay.item_fence(), foreign_command).await?;
    assert!(
        matches!(
            refused,
            Err(
                crate::gameplay_transport::spell_access_facts::AccessFactsError::Equipment(
                    crate::durability::character_equipment::EquipmentError::Rejected(
                        "activity cast command session"
                    )
                )
            )
        ),
        "ordinary cast cannot borrow the origin session command: {refused:?}"
    );
    // Drive a real ordinary spell, using current persisted class facts and
    // the existing fenced automatic-knowledge producer.
    let actor = admitted.runtime_actor.ok_or("activity caster")?;
    let index = destination
        .spells
        .spells
        .iter()
        .position(|spell| spell.name == "Light")
        .ok_or("production Light spell")?;
    let key = destination.spells.spells[index].key.clone();
    let current = crate::gameplay_transport::character_gameplay_fence::current(
        &destination,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("current automatic knowledge fence")?;
    destination
        .root
        .acquire_automatic_spells(
            destination.character,
            destination.holder,
            current,
            &std::collections::BTreeSet::from([key]),
        )
        .await
        .map_err(debug)?;
    let crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. } =
        crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            destination.root,
            destination.character,
            current.character_id,
        )
        .await
    else {
        return Err("actual classed activity cast facts".into());
    };
    let learned = destination
        .root
        .read_character_known_spells(destination.character, current.character_id)
        .await
        .map_err(debug)?;
    {
        let runtime = destination.runtime.lock().await;
        let mut states = destination.states.spell_states.lock().await;
        let mut state = crate::spell::cast::PlayerSpellState::new(facts, 0, 0)
            .ok_or("native activity cast state")?;
        state.load_owned_build(&cast.build).map_err(debug)?;
        state
            .enable_owned_training(
                &cast.build,
                native
                    .training_formula()
                    .ok_or("production training formula")?,
            )
            .map_err(debug)?;
        state
            .make_playable(destination.owner_now())
            .map_err(debug)?;
        state.install_learned_spells(learned);
        *states
            .get_mut(&runtime, actor, admitted.game_session_id)
            .ok_or("activity spell state")? = state;
    }
    let (_, before_cast) = destination
        .observe_vitals(actor, admitted.game_session_id)
        .await
        .ok_or("before Light vitals")?;
    let intent = crate::gameplay_transport::actor_spell::SpellCastIntent {
        spell: std::num::NonZeroU32::new(u32::try_from(index)? + 1).ok_or("Light index")?,
        target: crate::gameplay_transport::actor_spell::SpellTarget::None,
        aim_at_target: false,
    };
    let outcome = destination
        .complete_actor_spell(actor, admitted.game_session_id, 8, intent)
        .await;
    assert!(
        matches!(
            outcome,
            Some(crate::gameplay_transport::actor_spell::SpellCastOutcome {
                disposition: crate::gameplay_transport::actor_spell::SpellCastDisposition::Cast,
                ..
            })
        ),
        "actual source Instance Light cast: {outcome:?}"
    );
    let (_, after_cast) = destination
        .observe_vitals(actor, admitted.game_session_id)
        .await
        .ok_or("after Light vitals")?;
    assert_eq!(
        before_cast.mana.checked_sub(after_cast.mana),
        Some(20),
        "native Light mana cost"
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    super::m26_rotten_room_taints::check(&destination, admitted, origin_session).await?;
    super::m26_rotten_wave_production::check(&destination, admitted).await?;
    eprintln!(
        "M26_STATIC_INSTANCE POSTGRES_CASE_COMPLETED duration={:?} success=true",
        started.elapsed()
    );
    drop(destination);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await?;
    Ok(())
}

// A busy holder is retryable; negative cases still require their concrete fence refusal.
async fn read_activity_lifecycle(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: &crate::durability::character_progression::CurrentCharacterGameplayFence,
    digest: [u8; 32],
) -> TestResult<
    Result<
        crate::durability::character_equipment::RawLifecycleDurableFacts,
        crate::durability::character_equipment::EquipmentError,
    >,
> {
    use crate::durability::{DurabilityError, character_equipment::EquipmentError};
    use crate::gameplay_transport::connection::AdmissionRefusal;
    crate::gameplay_transport::AdmissionRetryBudget::for_command(owner.root)
        .retry_unavailable(async || {
            match owner
                .root
                .read_lifecycle_durable_facts(owner.character, owner.holder, fence, digest)
                .await
            {
                Err(EquipmentError::Durability(
                    DurabilityError::Unavailable
                    | DurabilityError::RootUnavailable
                    | DurabilityError::RootPassDeadlineExceeded,
                )) => Err(AdmissionRefusal::Unavailable),
                result => Ok(result),
            }
        })
        .await
        .map_err(debug)
}

async fn read_activity_cast(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
    command: CommandRef,
    digest: [u8; 32],
) -> TestResult<
    Result<
        crate::durability::character_equipment::RawCastDurableFacts,
        crate::durability::character_equipment::EquipmentError,
    >,
> {
    use crate::durability::{DurabilityError, character_equipment::EquipmentError};
    use crate::gameplay_transport::connection::AdmissionRefusal;
    crate::gameplay_transport::AdmissionRetryBudget::for_command(owner.root)
        .retry_unavailable(async || {
            match owner
                .root
                .read_cast_durable_facts(owner.character, owner.holder, fence, command, digest)
                .await
            {
                Err(EquipmentError::Durability(
                    DurabilityError::Unavailable
                    | DurabilityError::RootUnavailable
                    | DurabilityError::RootPassDeadlineExceeded,
                )) => Err(AdmissionRefusal::Unavailable),
                result => Ok(result),
            }
        })
        .await
        .map_err(debug)
}

// Exercise the real ordinary-cast reader in its owner's physical SQL transaction.
async fn read_owned_activity_cast(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
    command: CommandRef,
) -> TestResult<Result<(), crate::gameplay_transport::spell_access_facts::AccessFactsError>> {
    use crate::durability::{DurabilityError, character_equipment::EquipmentError};
    use crate::gameplay_transport::{
        connection::AdmissionRefusal,
        spell_access_facts::{AccessFactsError, read_owned_cast_facts_in_transaction},
    };
    crate::gameplay_transport::AdmissionRetryBudget::for_command(owner.root)
        .retry_unavailable(async || {
            let pass = owner.root.try_issue_semantic_pass().map_err(|_| AdmissionRefusal::Unavailable)?;
            let mut context = owner;
            let result = pass.run_with_context(&mut context, |holder, deadline, owner| Box::pin(async move {
                let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(holder, deadline).await?;
                let active = owner.active_generation.ok_or(DurabilityError::Unavailable)?;
                let result = read_owned_cast_facts_in_transaction(&mut tx, owner.root, owner.character,
                    owner.holder, fence, command, active).await.map(|_| ());
                // This is a read-only transaction, as in the production fresh cast recheck.
                drop(tx);
                Ok(result)
            })).await.map_err(|_| AdmissionRefusal::Unavailable)?;
            match result {
                Err(AccessFactsError::Durability(DurabilityError::Unavailable
                    | DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded))
                | Err(AccessFactsError::Equipment(EquipmentError::Durability(DurabilityError::Unavailable
                    | DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded))) =>
                    Err(AdmissionRefusal::Unavailable),
                result => Ok(result),
            }
        }).await.map_err(debug)
}
