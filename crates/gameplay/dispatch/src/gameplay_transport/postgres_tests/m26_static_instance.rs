//! Actual source handoff proves a static Instance needs no Channel Item initializer.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::native_gameplay::NativeGameplayInput;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::{SeamRooms, item_fence_of};
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
    assert!(
        destination.ensure_source_map_initialized().await,
        "the real static Instance is immediately ready for the Movement owner"
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
    assert!(started.elapsed().as_secs_f64() > 0.01);
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
