//! Source-qualified Spite room: real entry, native pursuit, death, Soul War state and return.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::native_gameplay::NativeGameplayInput;
use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityReturnReason};
use crate::durability::character_equipment::{EquipmentError, MovementEquipmentSnapshot};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    CharacterLease, CommandId, CommandRef, MovementLocalPosition, RuntimeScopeRefV1,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use crate::gameplay_transport::native_activity::NativeActivityOutcome;
use crate::gameplay_transport::{SeamRooms, item_fence_of};
const SPITE_KEY: &str = "oteryn:creature.goshnar_s_spite";
use oteryn_protocol_oteryn::boss_encounter as wire;
use std::time::{Duration, Instant};

#[test]
fn m26_spite_production_entry_fight_quest_reward_custody_and_return() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| scenario(admin).await)
}

async fn scenario(admin: String) -> TestResult {
    let started = Instant::now();
    super::reset();
    let h = Harness::create_with_admission_seed(admin, "m26_spite_chain", true, false).await?;
    let evidence = super::house_transfer::RecoverySource::start(&h).await?;
    super::house_transfer::prepare_owned_session(&h).await?;
    let world = super::fixture::fence(1)?.runtime_scope.world_id();
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/spells.manifest.json");
    let mut input = NativeGameplayInput::from_manifest(&manifest).map_err(debug)?;
    // Rebind only the harness's historical Character interpretation. Keep every
    // production creature, source region, hook, room policy and reward unchanged.
    input
        .build_training
        .as_mut()
        .ok_or("training")?
        .content_revision = "content-1".into();
    let (controller, source_pin) =
        crate::content::native_gameplay::test_support::try_activate_input(world, &input)
            .map_err(debug)?;
    let active = controller.active().ok_or("active production content")?;
    let native = active
        .native_gameplay()
        .ok_or("native production content")?;
    let room = source_pin.qualified_room().clone();
    let candidate = crate::content::bosses::activity_template::captured_boss_levers()
        .map_err(debug)?
        .into_iter()
        .find(|row| row.creature_key.as_deref() == Some(SPITE_KEY))
        .ok_or("Spite source candidate")?;
    let template = controller
        .qualify_activity_template(&room, &room, "content-1", &candidate)
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
    let pin = source_pin.into_channel_pin();
    let pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.server_artifact_digest(),
        pin.client_artifact_digest(),
        pin.frame_binding_digest(),
        pin.map_revision_digest(),
        (lever.x, lever.y, i16::try_from(lever.z)?),
    );
    let deps = Dependencies::with_content_and_capacity(
        pin.clone(),
        room,
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
    owner.active_generation = Some(active);
    owner.rooms = SeamRooms::new(Some(&deps.room), Some(&deps.room));
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
    super::boss_activity::issue_content_and_reset(&h, &pin).await?;
    ActivityHandoffOwner::new(h.root.clone())
        .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
        .await?;
    owner
        .activity_runtimes
        .register_template(&template)
        .map_err(|_| "register source template")?;
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
    let mut destination = owner
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
    let actor = admitted.runtime_actor.ok_or("Instance player")?;
    let boss = {
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
    let before = destination
        .runtime
        .lock()
        .await
        .read_actor_position(boss)
        .map_err(debug)?
        .position();
    {
        use crate::spell::source_map_initialization::CurrentNativeMapInitialization;
        let runtime = destination.runtime.lock().await;
        let room = destination
            .rooms()
            .gate_source_map_initialization()
            .ok_or("actual bounded source Instance map")?;
        assert!(
            room.source_world()
                .ok_or("source Instance")?
                .pending_mutable_placements()
                .is_empty()
        );
        assert!(
            CurrentNativeMapInitialization::qualify_static_instance(&runtime, room, native).is_ok()
        );
        assert!(
            CurrentNativeMapInitialization::qualify_current_binding(&runtime, room, native)
                .is_err(),
            "a static Instance never borrows Channel Item custody"
        );
        assert!(
            CurrentNativeMapInitialization::qualify_static_instance(&runtime, &deps.room, native)
                .is_err(),
            "the full source map cannot borrow the bounded room's empty-placement proof"
        );
    }
    // Match the production movement preflight: its bounded PvP read also
    // recovers a busy durability holder after the handoff's initial snapshots.
    assert!(
        destination
            .refresh_pvp_locks(actor, admitted.game_session_id)
            .await,
        "production activity movement preflight"
    );
    let equipment_fence = admitted.item_fence.ok_or("movement equipment fence")?;
    assert_eq!(
        Some(equipment_fence),
        item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0
        ),
        "the handed-off transport retains its actual current durable equipment fence"
    );
    let equipment =
        read_activity_equipment(&destination, &equipment_fence, native.source_digest()).await?;
    assert!(
        equipment.is_ok(),
        "actual activity participation admits its persisted equipment read: {equipment:?}"
    );
    let wrong_content = read_activity_equipment(&destination, &equipment_fence, [0xa5; 32]).await?;
    assert!(
        matches!(
            wrong_content,
            Err(EquipmentError::Rejected(
                "activity equipment destination Content pin"
            ))
        ),
        "the activity cannot substitute another equipment policy generation: {wrong_content:?}"
    );
    let mut stale_equipment_fence = equipment_fence;
    stale_equipment_fence.character_lease_generation += 1;
    let stale_lease =
        read_activity_equipment(&destination, &stale_equipment_fence, native.source_digest())
            .await?;
    assert!(
        matches!(
            stale_lease,
            Err(EquipmentError::Rejected(
                "activity equipment current lifecycle fence"
            ))
        ),
        "a stale activity lease grants no equipment read: {stale_lease:?}"
    );
    assert!(
        destination.ensure_source_map_initialized().await,
        "static source map admission"
    );
    assert!(
        destination
            .refresh_pvp_locks(actor, admitted.game_session_id)
            .await,
        "the actual activity retains its fenced combat locks"
    );
    {
        let runtime = destination.runtime.lock().await;
        assert!(
            runtime
                .player_control_facts(actor, admitted.game_session_id)
                .map_err(debug)?
                .control_loss
                .is_none()
        );
    }
    assert!(
        destination
            .read_movement_equipment(actor, admitted.game_session_id)
            .await
            .is_some(),
        "the Movement owner consumes the real equipment facts"
    );
    // The source landing is seven rows south of Spite, outside the player's
    // north-facing view (six rows). Enter the fight through the actual paced
    // Movement owner; the boss must never acquire a player it cannot perceive.
    let entry = destination
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert!(!crate::ai_think::targeting::sees(entry, before));
    let (step, _) = destination
        .ordinary_paced_step(
            actor,
            admitted.game_session_id,
            oteryn_protocol_oteryn::world_spatial::StepDirection::North,
            None,
            None,
        )
        .await;
    assert_eq!(
        step.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
        "the source entry admits the real northward movement into the fight"
    );
    let visible_from = destination
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(visible_from.y, entry.y - 1);
    assert!(crate::ai_think::targeting::sees(visible_from, before));
    let now = destination.owner_now();
    assert!(
        destination.spell_lane_permit().await.is_some(),
        "the actual Instance has its fenced spell lane"
    );
    {
        let runtime = destination.runtime.lock().await;
        let states = destination.states.spell_states.lock().await;
        let attack = destination.states.attack.lock().await;
        let census = runtime.positioned_melee_census().map_err(debug)?;
        let room = destination
            .rooms()
            .gate_monster_ai()
            .ok_or("Instance AI room")?;
        let targets = crate::gameplay_transport::monster_ai_cycle::melee_targets(
            room,
            &runtime,
            &states,
            &attack,
            &census,
            now.get(),
        );
        assert_eq!(
            targets.len(),
            1,
            "the actual Instance participant is a native AI target"
        );
        assert!(
            !targets[0].4,
            "the source entry participant is not protected from monsters"
        );
    }
    for tick in 0..=8 {
        destination
            .drain_monster_melee_at(
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
                    now.get() + tick * 1_000_000,
                ),
            )
            .await;
        assert!(
            destination
                .states
                .spell_states
                .lock()
                .await
                .monster_melee
                .creature_ai
                .state(boss)
                .is_some(),
            "the native scheduler admits the source boss"
        );
    }
    let after = destination
        .runtime
        .lock()
        .await
        .read_actor_position(boss)
        .map_err(debug)?
        .position();
    assert_ne!(
        before, after,
        "the source boss pursues the actual admitted participant"
    );
    destination.clock_origin -= Duration::from_millis(8_002);
    let now = destination.owner_now();
    let boss_policy = native
        .policies()
        .policies()
        .iter()
        .find(|row| row.definition_key == SPITE_KEY)
        .ok_or("source boss policy")?;
    let rewards = native.reward_table().row(SPITE_KEY).map_err(debug)?;
    assert!(
        !rewards.loot_table.entries.is_empty(),
        "source personal loot table"
    );
    {
        let mut runtime = destination.runtime.lock().await;
        let target = runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find(|(target, _, _)| {
                runtime
                    .companion_snapshot(*target)
                    .is_ok_and(|row| row.state.policy.definition_key == SPITE_KEY)
            })
            .map(|row| row.0)
            .ok_or("living source boss")?;
        let current = admitted.item_fence.ok_or("Instance fence")?;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(current.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: current.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "m26-spite-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"m26-spite-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: boss_policy.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(receipt.effects.iter().any(|effect| {
            effect.target == target
                && effect
                    .health
                    .is_some_and(|health| health.health_before > 0 && health.health_after == 0)
        }));
        let attack = destination.states.attack.try_lock().map_err(debug)?;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
        assert!(runtime.retains_corpse_projection(target));
    }
    assert_eq!(
        destination
            .drain_kill_rewards(admitted.game_session_id)
            .await,
        DrainOutcome::Drained
    );
    let state: (bool, i16, bool) = sqlx::query_as(
        "SELECT teleport_access,defeated_bosses,first_taint_at IS NOT NULL FROM game_character_soul_war WHERE character_id=encode($1,'hex')::uuid"
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        state,
        (true, 4, true),
        "actual native death awards Spite's first taint"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_contributors")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_items")
            .fetch_one(&h.pool)
            .await?
            > 0,
        "actual source personal loot is retained"
    );
    eprintln!("M26_SPITE_STAGE native_death_and_soul_war_quest_settlement");
    let activity: (i16, i64, i64) =
        sqlx::query_as("SELECT state,terminal_at,exit_deadline FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(activity.0, 2);
    assert_eq!(
        activity.2,
        activity.1 + 60_000,
        "unmodified source after-kill policy"
    );
    assert_eq!(destination.runtime.lock().await.visible_corpses().len(), 1);
    let RuntimeScopeRefV1::Instance { instance_id, .. } = instance else {
        return Err("actual corpse Instance".into());
    };
    assert_eq!(sqlx::query_scalar::<_,i64>(
        "SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.instance_id=encode($1,'hex')::uuid AND g.channel_id IS NULL AND i.definition_production_key=$2")
        .bind(instance_id.as_slice()).bind(&rewards.corpse_item.as_ref().ok_or("corpse profile")?.production_key).fetch_one(&h.pool).await?,1,
        "the source corpse is durably placed in this exact Instance");
    // Wait for the real durable source deadline; never shorten the captured policy.
    tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let due:bool=sqlx::query_scalar("SELECT exit_deadline <= floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities").fetch_one(&h.pool).await?;
            if due { return Ok::<(),sqlx::Error>(()); }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }).await??;
    let result = destination
        .return_due_activity(admitted, ActivityReturnReason::Defeated)
        .await
        .map_err(|_| "source deadline return")?;
    let NativeActivityOutcome::Committed(commit) = result else {
        return Err(format!("return refused: {result:?}").into());
    };
    let returned = commit.connections.first().ok_or("returned session")?.1;
    owner
        .publish_activity_commit(&commit)
        .map_err(|_| "publish source return")?;
    assert!(matches!(
        returned.item_fence.ok_or("return fence")?.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    let exit = template.exit_position();
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(returned.runtime_actor.ok_or("returned actor")?)
            .map_err(debug)?
            .position(),
        MovementLocalPosition {
            x: exit.x,
            y: exit.y,
            floor: i16::try_from(exit.z)?
        }
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_activity_runtime_returns WHERE reason=3 AND command_id IS NULL").fetch_one(&h.pool).await?,1);
    assert_eq!(
        sqlx::query_scalar::<_, i16>("SELECT state FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?,
        4
    );
    eprintln!(
        "M26_SPITE_CHAIN POSTGRES_CASE_COMPLETED entry_cadence_death_settlement_return duration={:?}",
        started.elapsed()
    );
    Ok(())
}

// Retry only unavailable read holders; negative cases must observe the actual
// policy/fence refusal, never pass because an unrelated holder was busy.
async fn read_activity_equipment(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
    digest: [u8; 32],
) -> TestResult<Result<MovementEquipmentSnapshot, EquipmentError>> {
    use crate::durability::DurabilityError;
    use crate::gameplay_transport::connection::AdmissionRefusal;
    crate::gameplay_transport::AdmissionRetryBudget::for_command(owner.root)
        .retry_unavailable(async || {
            match owner
                .root
                .read_movement_equipment(owner.character, owner.holder, fence, digest)
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
