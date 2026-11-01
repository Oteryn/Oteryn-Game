//! A real native reward kill crosses a sourced level threshold. Its live level
//! and soul effects stay pending until the real Instance return moves them to
//! the Channel owner, whose ordinary cadence applies each exactly once.
use super::{Harness, TestResult, debug, fixture};
use crate::content::native_gameplay::{
    CreatureProfilesDocument, NativeGameplayInput, PinnedGameplayBytes,
};
use crate::content::{
    ContentActivationController, NativeEntryActivationIssuance, NativeEntryContentPin,
    NodeBootQuiescence, ProjectV2AuthoringProfileData, activate_native_entry_room_with_gameplay,
};
use crate::durability::activity_handoff::ActivityReturnReason;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::character_progression::{
    CharacterProgressionState, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    CommandId, CommandRef, CompiledCreaturePolicy, GameSessionId, RuntimeScopeRefV1, WorldId,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use crate::gameplay_transport::native_activity::NativeActivityOutcome;
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
use oteryn_gameplay_characters::soul::{AppliedKillSoulFacts, SoulKillHook, soul_kill_participant};
use oteryn_simulation_determinism::ExactI64;
use sha2::Digest;

pub(super) fn generation(
    world: WorldId,
    content: &str,
) -> TestResult<(ContentActivationController, NativeEntryContentPin)> {
    let manifest = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server"
    ))
    .join("../../content/spells.manifest.json");
    let mut supplied = NativeGameplayInput::from_manifest(&manifest).map_err(debug)?;
    // Same pre-activation interpretation as death_ml_loss_composed. The real
    // Character fixture is content-1; training under another revision refuses.
    supplied
        .build_training
        .as_mut()
        .ok_or("pinned training")?
        .content_revision = content.into();
    // Reuse the explicit reward-boss fixture's sole classification change. HP,
    // XP, corpse, loot chances, quantities and Item definitions remain sourced.
    let mut creatures: CreatureProfilesDocument =
        serde_json::from_slice(&supplied.creature_profiles.bytes)?;
    let rat = creatures
        .records
        .iter_mut()
        .find(|record| record.profile.target.key == "oteryn:creature.rat")
        .ok_or("source rat")?;
    let ProjectV2AuthoringProfileData::Creature(profile) = &mut rat.profile.data else {
        return Err("source rat Creature profile".into());
    };
    profile
        .details
        .as_mut()
        .ok_or("source rat details")?
        .system_eligibility
        .reward_boss = true;
    let bytes = serde_json::to_vec(&creatures)?;
    supplied.creature_profiles = PinnedGameplayBytes {
        sha256: sha2::Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        bytes,
    };
    let room =
        crate::content::qualify_selected_native_gameplay_room(world, &supplied).map_err(debug)?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = ContentActivationController::new();
    let pin = activate_native_entry_room_with_gameplay(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &supplied,
    )
    .map_err(debug)?;
    Ok((controller, pin))
}

async fn fence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult<CurrentCharacterGameplayFence> {
    owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "current award fence")?
        .ok_or_else(|| "current award session".into())
}

async fn progression(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<CharacterProgressionState> {
    owner
        .root
        .read_character_progression(
            owner.character,
            admitted.item_fence.ok_or("award Character")?.character_id,
        )
        .await
        .map_err(debug)?
        .ok_or_else(|| "actual progression".into())
}

async fn facts(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<CharacterCastFacts> {
    match load_character_cast_facts(
        owner.root,
        owner.character,
        admitted.item_fence.ok_or("facts Character")?.character_id,
    )
    .await
    {
        CastFactsLoad::Ready { facts, .. } => Ok(facts),
        _ => Err("actual source cast facts".into()),
    }
}

pub(super) async fn prepare_source(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    owner.admit_progression_session(admitted).await;
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("real progression admission")?;
    let formula = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("qualified training formula")?;
    let before_build = DurableBuildState::default();
    let chosen = h
        .root
        .commit_character_build(
            owner.character,
            owner.holder,
            fence(owner, admitted.game_session_id).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(fixture::id(226)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                after: convert_vocation(formula, &before_build, "knight").map_err(debug)?,
                before: before_build,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
    let before = progression(owner, *admitted).await?;
    let next = binding
        .policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level > before.level)
        .ok_or("next sourced level threshold")?;
    let total = next
        .minimum_experience
        .get()
        .checked_sub(1)
        .ok_or("threshold predecessor")?;
    let amount = total
        .checked_sub(before.total_experience.get())
        .ok_or("source XP setup delta")?;
    assert!(amount > 0);
    let seed = h
        .root
        .commit_character_experience(
            owner.character,
            owner.holder,
            fence(owner, admitted.game_session_id).await?,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(fixture::id(227))
                    .map_err(debug)?,
                amount: ExactI64::new(amount),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await
        .map_err(debug)?;
    assert!(matches!(seed, ExperienceCommitOutcome::Committed(_)));
    let prepared = progression(owner, *admitted).await?;
    assert_eq!(prepared.level, before.level);
    assert_eq!(prepared.total_experience.get(), total);
    owner.revision_sequencer = CharacterRevisionSequencer::new().into();
    let facts = facts(owner, *admitted).await?;
    let actor = admitted.runtime_actor.ok_or("source actor")?;
    let now = owner.owner_now();
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    assert!(
        states
            .initialize(
                &runtime,
                actor,
                admitted.game_session_id,
                facts,
                (0, 0),
                now
            )
            .is_some()
    );
    let state = states
        .get_mut(&runtime, actor, admitted.game_session_id)
        .ok_or("actual source vitals")?;
    // An ordinary persisted resting-state input starts this isolated actor one
    // soul below its sourced maximum; no pending award or hook is manufactured.
    state
        .restore_bed_rest(
            [
                facts.max_soul.checked_sub(1).ok_or("source soul maximum")?,
                0,
                0,
                0,
            ],
            now.get(),
        )
        .ok_or("restored source soul")?;
    assert_eq!(state.soul_window(), None);
    Ok(())
}

async fn live(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<PlayerSpellState> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    states
        .get(
            &runtime,
            admitted.runtime_actor.ok_or("live actor")?,
            admitted.game_session_id,
        )
        .cloned()
        .ok_or_else(|| "current live state".into())
}

async fn kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &CompiledCreaturePolicy,
) -> TestResult<OwnerCombatBatch> {
    let fence = admitted.item_fence.ok_or("Instance award fence")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("active native rewards")?;
    let now = owner.owner_now();
    let mut runtime = owner.runtime.lock().await;
    assert_eq!(runtime.binding().runtime_scope(), fence.runtime_scope);
    let target = runtime
        .visible_entities()
        .creatures
        .first()
        .ok_or("actual native boss")?
        .actor;
    let batch = OwnerCombatBatch {
        caster: admitted.runtime_actor.ok_or("actual caster")?,
        attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
            .map_err(debug)?,
        current_lease_generation: fence.character_lease_generation,
        command: CommandRef::new(admitted.game_session_id, CommandId::new(92).map_err(debug)?),
        occurrence: SpellOccurrenceBinding {
            id: "boss-activity:pending-effects-lethal".into(),
            revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
        },
        binding: b"boss-activity-pending-effects-lethal".to_vec(),
        anchor: None,
        now_us: Some(now),
        now_ms: now.get() / 1_000,
        effects: vec![OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                magnitude: policy.maximum_health,
            },
        }],
        deferred: None,
    };
    let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
    let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
    assert!(receipt.applied);
    assert!(receipt.effects.iter().any(|effect| effect.target == target
        && effect.health.is_some_and(|health| health.health_after == 0)));
    let attack = owner.states.attack.lock().await;
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
        now.get() / 1_000,
    );
    assert!(attack.kills().has_session(admitted.game_session_id));
    Ok(batch)
}

async fn replay_kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    batch: &OwnerCombatBatch,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("replay native rewards")?;
    {
        let mut runtime = owner.runtime.lock().await;
        let staged = runtime.stage_spell_batch(batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(!receipt.applied);
        let attack = owner.states.attack.lock().await;
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
            batch.now_ms,
        );
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    Ok(())
}

async fn return_after_defeat(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<AdmittedSession> {
    let (state, deadline, now): (i16, i64, i64) = sqlx::query_as(
        "SELECT state,exit_deadline,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities",
    ).fetch_one(&h.pool).await?;
    assert_eq!(state, 2);
    tokio::time::sleep(std::time::Duration::from_millis(
        u64::try_from(deadline.saturating_sub(now).max(0))? + 5,
    ))
    .await;
    let result = instance
        .return_due_activity(admitted, ActivityReturnReason::Defeated)
        .await
        .map_err(|_| "actual after-kill return")?;
    let NativeActivityOutcome::Committed(commit) = result else {
        return Err(format!("after-kill return refused: {result:?}").into());
    };
    let next = commit
        .connections
        .first()
        .ok_or("actual return successor")?
        .1;
    assert!(matches!(
        next.item_fence.ok_or("Channel successor")?.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    assert_ne!(next.game_session_id, admitted.game_session_id);
    origin
        .publish_activity_commit(&commit)
        .map_err(|_| "publish return")?;
    Ok(next)
}

pub(super) async fn assert_award_follows_return(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &CompiledCreaturePolicy,
) -> TestResult {
    let character = admitted.item_fence.ok_or("award Character")?.character_id;
    let before = progression(instance, admitted).await?;
    let before_live = live(instance, admitted).await?;
    assert_eq!(before_live.character_facts().level, before.level);
    assert_eq!(before_live.soul_window(), None);
    let batch = kill(instance, admitted, policy).await?;
    assert_eq!(
        instance.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let awarded = progression(instance, admitted).await?;
    assert!(
        awarded.level > before.level,
        "the actual native kill crosses the sourced threshold"
    );
    let actual_gain = awarded
        .total_experience
        .get()
        .checked_sub(before.total_experience.get())
        .ok_or("native XP delta")?;
    let hook =
        soul_kill_participant(AppliedKillSoulFacts::new(before.level, actual_gain).map_err(debug)?)
            .hook();
    assert!(matches!(hook, SoulKillHook::OpenWindow { .. }));
    assert!(instance.states.level_refresh.is_marked(character));
    assert_eq!(
        live(instance, admitted).await?,
        before_live,
        "the normal live cadence has not applied the award yet"
    );
    replay_kill(instance, admitted, &batch).await?;
    let replayed = progression(instance, admitted).await?;
    assert_eq!(
        (replayed.level, replayed.total_experience),
        (awarded.level, awarded.total_experience)
    );
    let next = return_after_defeat(h, origin, instance, admitted).await?;
    assert!(!instance.states.level_refresh.is_marked(character));
    assert!(origin.states.level_refresh.is_marked(character));
    assert_eq!(
        live(origin, next).await?,
        before_live,
        "return moves pending state before ordinary cadence"
    );
    apply_and_check(origin, next, &before_live, awarded.level).await?;
    let final_xp = progression(origin, next).await?;
    assert_eq!(
        (final_xp.level, final_xp.total_experience),
        (awarded.level, awarded.total_experience)
    );
    eprintln!("BOSS_ACTIVITY_NATIVE_AWARD_LEVEL_SOUL_RETURN_ONCE POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn apply_and_check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    before: &PlayerSpellState,
    expected_level: u32,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("returned actor")?;
    let expected_facts = facts(owner, admitted).await?;
    assert_eq!(expected_facts.level, expected_level);
    let expected = before
        .after_level_up(expected_facts)
        .ok_or("sourced level-up vitals")?;
    owner
        .refresh_level_up_vitals(actor, admitted.game_session_id)
        .await;
    owner
        .apply_soul_kills(actor, admitted.game_session_id)
        .await;
    let applied = live(owner, admitted).await?;
    assert_eq!(applied.character_facts(), expected_facts);
    assert_eq!(
        applied.vitals(),
        expected.vitals(),
        "exact sourced current pools and maxima"
    );
    assert!(
        applied.soul_window().is_some(),
        "the committed kill's pending soul hook followed the return"
    );
    assert!(
        !owner
            .states
            .level_refresh
            .is_marked(admitted.item_fence.ok_or("applied Character")?.character_id)
    );
    owner
        .refresh_level_up_vitals(actor, admitted.game_session_id)
        .await;
    owner
        .apply_soul_kills(actor, admitted.game_session_id)
        .await;
    assert_eq!(
        live(owner, admitted).await?,
        applied,
        "repeated cadence must not refill pools or renew the soul window"
    );
    Ok(())
}
