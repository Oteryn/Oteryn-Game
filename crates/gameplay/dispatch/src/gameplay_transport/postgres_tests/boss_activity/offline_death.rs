//! A real transferred actor dies after losing its socket. The existing native
//! death, selected AoL burn, temple return and terminal release all run through
//! their production owners; no death receipt or returned session is fabricated.
use super::{Harness, TestResult, debug, fixture};
use crate::durability::activity_handoff::{ActivityRefusal, ActivityReturnReason};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
    DurableBuildState, convert_vocation, prepare_character_build_in_transaction,
};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    AuthenticatedTransportRefV1, ControlLossEpochRefV1, GameSessionId, GameSessionState,
    RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ControlLossResult, FreshAdmissionAuthority, ResumeAttempt,
};
use crate::gameplay_transport::native_activity::NativeActivityOutcome;
use oteryn_gameplay_progression::hit_training::SkillFamily;
use std::time::Duration;

pub(super) async fn prepare_source(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    owner.admit_progression_session(admitted).await;
    let progression = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("real production progression admission")?;
    let formula = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("qualified native training formula")?;
    let interpretation = h.root.read_character_root_interpretation(
        owner.character,
        admitted.item_fence.ok_or("source interpretation fence")?.character_id,
    ).await.map_err(|error| format!("offline-death root interpretation: {error:?}"))?
        .ok_or("source Character interpretation")?;
    assert_eq!(formula.content_revision(), interpretation.revisions()[2],
        "the activated training formula must match this fixture Character's independently stored content revision");
    let before = DurableBuildState::default();
    let chosen = h.root.commit_character_build(
        owner.character,
        owner.holder,
        current_fence(owner, admitted.game_session_id).await?,
        BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(fixture::id(211)).map_err(debug)?,
            cause: BuildCause::VocationChoice,
            after: convert_vocation(formula, &before, "monk").map_err(debug)?,
            before,
            pruned_stance: None,
        },
        formula,
    ).await.map_err(|error| format!("offline-death vocation setup: {error:?}"))?;
    assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
    let level_twenty = progression.policy.thresholds.iter()
        .find(|threshold| threshold.level == 20)
        .ok_or("source level twenty threshold")?.minimum_experience;
    let awarded = h.root.commit_character_experience(
        owner.character,
        owner.holder,
        current_fence(owner, admitted.game_session_id).await?,
        ExperienceAwardRequest {
            occurrence: ExperienceRewardOccurrence::from_bytes(fixture::id(212)).map_err(debug)?,
            amount: level_twenty,
            context: progression.context.clone(),
            policy_revision: progression.policy_revision.clone(),
            reward_revision: progression.reward_revision.clone(),
            policy: progression.policy.clone(),
        },
    ).await.map_err(|error| format!("offline-death experience setup: {error:?}"))?;
    assert!(matches!(awarded, ExperienceCommitOutcome::Committed(_)));
    // The explicit initial setup writes use the same durable owners as the
    // existing death fixture. Start the live revision writer after those writes.
    owner.revision_sequencer = CharacterRevisionSequencer::new().into();
    seed_amulet(h).await
}

async fn current_fence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult<crate::durability::character_progression::CurrentCharacterGameplayFence> {
    owner.current_quest_fence(session).await
        .map_err(|_| "current death fixture fence")?
        .ok_or_else(|| "current death fixture session".into())
}

async fn seed_amulet(h: &Harness) -> TestResult {
    super::super::item_look::seed_main_backpack(h, &[]).await?;
    // Preexisting custody is fixture data. Loss and every subsequent lifecycle
    // transition below run with the actual database owners and triggers enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(fixture::id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3057','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(fixture::id(75).as_slice()).bind(fixture::id(42).as_slice()).bind(fixture::id(76).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,7,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(fixture::id(41).as_slice()).bind(fixture::id(75).as_slice()).bind(fixture::id(42).as_slice()).bind(fixture::id(76).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn assert_offline_death(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    deadline: bool,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual Instance fence")?;
    assert!(matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. }));
    assert!(instance.progression_binding(admitted.game_session_id).is_some());
    assert_eq!(origin.lose_control(admitted, Duration::ZERO, None).await, ControlLossResult::Recorded);
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id).await?.0;
    assert_eq!(current.session_state(), GameSessionState::Reconnectable);
    assert!(current.current_transport().is_none());
    let epoch = current.current_control_loss_epoch().ok_or("real loss epoch")?;
    let death = train_and_kill(h, instance, admitted).await?;
    // A dead actor does not authorize an obsolete terminal trigger. Check this
    // before the valid trigger settles the first durable death.
    let stale = instance.return_due_activity(admitted, ActivityReturnReason::CapabilityMismatch {
        epoch: ControlLossEpochRefV1::new(epoch.get() + 1).map_err(debug)?,
    }).await.map_err(|_| "stale terminal trigger result")?;
    assert!(matches!(stale, NativeActivityOutcome::Refused(ActivityRefusal::AuthorityRejected)));
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_death_receipts")
        .fetch_one(&h.pool).await?, 0);
    let release_session = if deadline {
        due_return(h, origin, instance, admitted).await?
    } else {
        admitted.game_session_id
    };
    if let Err(error) = refuse_authenticated_mismatch(origin, release_session, if deadline { 214 } else { 213 }).await {
        diagnose_death(h, instance, admitted).await;
        return Err(error);
    }
    assert_complete(h, origin, instance, admitted, death).await?;
    eprintln!("BOSS_ACTIVITY_OFFLINE_DEATH_{} POSTGRES_CASE_COMPLETED", if deadline { "DEADLINE" } else { "MISMATCH" });
    Ok(())
}

async fn train_and_kill(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<[u8; 16]> {
    let actor = admitted.runtime_actor.ok_or("offline actor")?;
    let fence = admitted.item_fence.ok_or("offline fence")?;
    let build = h.root.read_character_build_state(owner.character, fence.character_id).await.map_err(debug)?;
    let formula = owner.active_generation.and_then(|active| active.native_gameplay())
        .and_then(|native| native.training_formula()).ok_or("active training")?;
    let request = {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(runtime.player_control_facts(actor, admitted.game_session_id).map_err(debug)?.control_loss.is_some());
        assert!(states.load_owned_build(&runtime, actor, admitted.game_session_id, &build));
        let state = states.get_mut(&runtime, actor, admitted.game_session_id).ok_or("offline live state")?;
        state.enable_owned_training(&build, formula).map_err(debug)?;
        state.train_skill(SkillFamily::Sword, owner.owner_now().get(), formula, |_| 1).map_err(debug)?;
        assert!(state.has_training_to_flush(), "offline death must flush the actual pending try");
        // Prepare a read-only probe from the real pending delta. It is never
        // retained or consumed by the actor; the death owner still saves it.
        state.prepare_training_checkpoint(owner.owner_now().get(), formula,
            BuildOccurrence::from_bytes(fixture::id(215)).map_err(debug)?, true)
            .map_err(debug)?.ok_or("real pending Instance training request")?.request
    };
    assert_build_scope_fences(h, owner, admitted, &request, formula).await?;
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let state = states.get(&runtime, actor, admitted.game_session_id).ok_or("unconsumed offline training")?;
    assert!(state.has_training_to_flush());
    assert!(state.pending_training_checkpoint().is_none());
    let hit = states.apply_creature_damage_native(&runtime, actor, admitted.game_session_id, 100_000, crate::foundation::owner_timer::SemanticTimeMicros::from_micros(owner.owner_now().get()))
        .ok_or("real native offline lethal")?;
    assert_eq!(hit.damage.health_after, 0);
    let death = states.player_death(&runtime, actor, admitted.game_session_id).ok_or("retained native death")?;
    assert_eq!(hit.death, Some(*death.occurrence.as_bytes()));
    Ok(*death.occurrence.as_bytes())
}

async fn assert_build_scope_fences(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    request: &BuildChangeRequest,
    formula: &dyn BuildFormula,
) -> TestResult {
    let current = current_fence(owner, admitted.game_session_id).await?;
    assert!(matches!(current.runtime_scope, RuntimeScopeRefV1::Instance { .. }));
    assert_eq!(request.cause, BuildCause::Training);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_build_receipts")
        .fetch_one(&h.pool).await?;
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_build_receipts WHERE build_occurrence_id=encode($1,'hex')::uuid")
        .bind(request.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?, 0);
    let stale = CurrentCharacterGameplayFence {
        scope_ownership_generation: ScopeOwnershipGeneration::new(current.scope_ownership_generation.get() + 1).map_err(debug)?,
        ..current
    };
    let foreign = CurrentCharacterGameplayFence {
        runtime_scope: super::super::activity_runtime::allocate_ready_instance(h, 216).await?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        ..current
    };
    assert_ne!(foreign.runtime_scope, current.runtime_scope);
    for (label, rejected) in [("stale Instance generation", stale), ("foreign Ready Instance", foreign)] {
        let outcome = h.root.commit_character_build(owner.character, owner.holder,
            rejected, request.clone(), formula).await;
        assert!(matches!(&outcome, Err(CharacterProgressionError::AuthorityRejected)),
            "standalone build writer accepted {label}: {outcome:?}");
        let mut tx = h.pool.begin().await?;
        let outcome = prepare_character_build_in_transaction(&h.root, &mut tx,
            owner.character, owner.holder, rejected, request.clone(), formula).await;
        tx.rollback().await?;
        let error = outcome.err();
        assert!(matches!(&error, Some(CharacterProgressionError::AuthorityRejected)),
            "composed build writer accepted {label}: {error:?}");
    }
    // The same genuine checkpoint and exact current authority also reach the
    // composed writer. Roll back its preparation: only the native death flush
    // below may commit this actor's pending training or consume its successor.
    let mut tx = h.pool.begin().await?;
    let prepared = prepare_character_build_in_transaction(&h.root, &mut tx,
        owner.character, owner.holder, current, request.clone(), formula).await;
    tx.rollback().await?;
    if let Err(error) = prepared {
        return Err(format!("current Instance training transaction refused: {error:?}").into());
    }
    // Equal before/after is a valid Logout shape. Instance support for Training
    // must not broaden the OFFLINE0 Channel terminal/admission contracts.
    let logout = BuildChangeRequest {
        cause: BuildCause::Logout,
        after: request.before.clone(),
        ..request.clone()
    };
    let outcome = h.root.commit_character_build(owner.character, owner.holder,
        current, logout.clone(), formula).await;
    assert!(matches!(&outcome, Err(CharacterProgressionError::InvalidInput)),
        "Instance Logout must remain outside the training change: {outcome:?}");
    let mut tx = h.pool.begin().await?;
    let outcome = prepare_character_build_in_transaction(&h.root, &mut tx,
        owner.character, owner.holder, current, logout, formula).await;
    tx.rollback().await?;
    let error = outcome.err();
    assert!(matches!(&error, Some(CharacterProgressionError::InvalidInput)),
        "composed Instance Logout unexpectedly admitted: {error:?}");
    assert_eq!(current_fence(owner, admitted.game_session_id).await?, current,
        "scope refusals and a rolled-back prepare must not advance Character authority");
    assert_eq!(h.root.read_character_build_state(owner.character, current.character_id).await.map_err(debug)?, request.before);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_build_receipts")
        .fetch_one(&h.pool).await?, receipts, "refused or rolled-back training writes leave no receipt");
    Ok(())
}

async fn due_return(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
) -> TestResult<GameSessionId> {
    let (deadline, now): (i64, i64) = sqlx::query_as(
        "SELECT deadline,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities",
    ).fetch_one(&h.pool).await?;
    tokio::time::sleep(Duration::from_millis(u64::try_from(deadline.saturating_sub(now).max(0))? + 5)).await;
    cancel_undecided_return(h, origin, instance, source).await?;
    let replay = tokio::time::timeout(Duration::from_secs(30), instance.return_due_activity(source, ActivityReturnReason::Timeout))
        .await?.map_err(|_| "same original Timeout retries retained Death")?;
    let NativeActivityOutcome::Committed(commit) = replay else {
        return Err(format!("retained Timeout retry refused: {replay:?}").into());
    };
    origin.publish_activity_commit(&commit).map_err(|_| "offline Death publication")?;
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(destination_game_session_id) FROM game_activity_runtime_returns WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let destination = GameSessionId::decode(&bytes).map_err(debug)?;
    let current = FreshAdmissionStore::from_root(h.root.clone()).current_session_at(destination).await?.0;
    assert_eq!(current.session_state(), GameSessionState::Reconnectable);
    assert!(matches!(current.current_runtime_scope(), RuntimeScopeRefV1::Channel { .. }));
    assert!(current.current_transport().is_none());
    Ok(destination)
}


async fn cancel_undecided_return(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
) -> TestResult {
    // Isolated test database only: hold the real return INSERT after the native
    // actor/spell tokens were extracted. Cancelling this owner future leaves an
    // undecided SQL attempt, without a fabricated receipt or production hook.
    sqlx::raw_sql("CREATE FUNCTION test_pause_activity_death_return() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.reason=5 THEN PERFORM pg_advisory_xact_lock(741953); END IF; RETURN NEW; END $$; CREATE TRIGGER test_pause_activity_death_return BEFORE INSERT ON game_activity_runtime_returns FOR EACH ROW EXECUTE FUNCTION test_pause_activity_death_return()")
        .execute(&h.pool).await?;
    let mut blocker = h.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(741953)").execute(&mut *blocker).await?;
    let waiting = async {
        loop {
            let blocked = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=0 AND objid=741953 AND NOT granted)")
                .fetch_one(&h.pool).await;
            if matches!(blocked, Ok(true)) { break; }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    let cancelled = tokio::time::timeout(Duration::from_secs(30),
        crate::gameplay_transport::first(origin.activity_lifecycle_pass(), waiting)).await?;
    if let Some(early) = cancelled {
        blocker.rollback().await?;
        sqlx::raw_sql("DROP TRIGGER test_pause_activity_death_return ON game_activity_runtime_returns; DROP FUNCTION test_pause_activity_death_return()")
            .execute(&h.pool).await?;
        diagnose_death(h, instance, source).await;
        return Err(format!("production deadline lifecycle returned {early:?} before reaching the actual return SQL barrier").into());
    }
    {
        let runtime = instance.runtime.lock().await;
        let states = instance.states.spell_states.lock().await;
        let actor = source.runtime_actor.ok_or("frozen source actor")?;
        assert!(runtime.player_control_facts(actor, source.game_session_id).is_err());
        assert!(states.player_death(&runtime, actor, source.game_session_id).is_none(),
            "retry must use its retained trigger, not a lost live death lookup");
    }
    blocker.rollback().await?;
    sqlx::raw_sql("DROP TRIGGER test_pause_activity_death_return ON game_activity_runtime_returns; DROP FUNCTION test_pause_activity_death_return()")
        .execute(&h.pool).await?;
    Ok(())
}

async fn refuse_authenticated_mismatch(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    nonce: u8,
) -> TestResult {
    let supported: Vec<_> = BUNDLE_WORLD_OFFERED_CAPABILITIES.iter().map(|offer| offer.id)
        .filter(|cap| *cap != oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_V1).collect();
    let credential = super::super::house_transfer::RecoverySource::token(nonce)?;
    let refused = tokio::time::timeout(Duration::from_secs(30), owner.resume_session(ResumeAttempt {
        game_session_id: session,
        recovery_material: credential.as_bytes(),
        transport: AuthenticatedTransportRefV1::decode(&[nonce; 16]).map_err(debug)?,
        last_applied_server_sequence: 0,
        supported_capabilities: &supported,
    })).await?;
    if !matches!(refused, Err(AdmissionRefusal::Rejected)) {
        return Err(format!("authenticated mismatched recovery: {refused:?}").into());
    }
    Ok(())
}

async fn assert_complete(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    instance: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
    death: [u8; 16],
) -> TestResult {
    type ReturnRow = (Vec<u8>, i16, Option<String>, Vec<u8>, bool, bool);
    let row: ReturnRow = sqlx::query_as("SELECT uuid_send(destination_game_session_id),reason,command_id::text,uuid_send(death_occurrence_id),release_transport_ref IS NULL,release_loss_epoch IS NULL FROM game_activity_runtime_returns WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((row.1, row.2, row.3, row.4, row.5), (5, None, death.to_vec(), true, true));
    let returned = GameSessionId::decode(&row.0).map_err(debug)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    for session in [source.game_session_id, returned] {
        assert_eq!(store.current_session_at(session).await?.0.session_state(), GameSessionState::Terminal);
    }
    let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_death_receipts),(SELECT count(*) FROM game_player_death_item_records),(SELECT count(*) FROM game_character_pending_respawns),(SELECT count(*) FROM game_boss_room_participants WHERE returned_at IS NULL)")
        .fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1, 0, 0));
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_player_death_items_complete(encode($1,'hex')::uuid)")
        .bind(death.as_slice()).fetch_one(&h.pool).await?);
    let amulet: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(fixture::id(75).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(amulet, (0, 2), "existing AoL is burned exactly once");
    let build = h.root.read_character_build_state(origin.character, source.item_fence.ok_or("source fence")?.character_id).await.map_err(debug)?;
    assert_eq!(build.skills()[2].1, 1, "pending sword training survived the offline death flush and AoL protection");
    assert!(origin.runtime.lock().await.activity_participant_sessions().map_err(debug)?.is_empty());
    assert!(instance.runtime.lock().await.player_control_facts(source.runtime_actor.ok_or("old actor")?, source.game_session_id).is_err());
    // Re-running the real lifecycle is idempotent and retires the now empty
    // private room. No old Instance actor/session becomes playable again.
    origin.activity_lifecycle_pass().await.map_err(|_| "empty room retirement")?;
    origin.activity_lifecycle_pass().await.map_err(|_| "retirement replay")?;
    let fence = source.item_fence.ok_or("old scope")?;
    assert!(origin.activity_runtimes.get(fence.runtime_scope, fence.scope_ownership_generation.get()).is_none());
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_activity_runtime_returns").fetch_one(&h.pool).await?, 1);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_player_death_item_records").fetch_one(&h.pool).await?, 1);
    Ok(())
}

/// Failure-only observations keep a fixture refusal from hiding the failed
/// production stage. Idempotent retries remain genuine owners and this helper
/// never changes a failed test into a pass or invents a death/return receipt.
async fn diagnose_death(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) {
    let Some(actor) = admitted.runtime_actor else { return; };
    let counts = sqlx::query_as::<_, (i64,i64,i64,i64)>("SELECT (SELECT count(*) FROM game_character_death_receipts),(SELECT count(*) FROM game_character_pending_respawns),(SELECT count(*) FROM game_activity_runtime_returns),(SELECT count(*) FROM game_player_death_item_records)")
        .fetch_one(&h.pool).await;
    eprintln!("OFFLINE_DEATH_DIAG before_counts={counts:?}");
    eprintln!("OFFLINE_DEATH_DIAG fence={:?} binding={}", owner.current_quest_fence(admitted.game_session_id).await,
        owner.progression_binding(admitted.game_session_id).is_some());
    eprintln!("OFFLINE_DEATH_DIAG home_temple={:?}", owner.home_temple(admitted.game_session_id).await);
    let death = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let death = states.player_death(&runtime, actor, admitted.game_session_id);
        eprintln!("OFFLINE_DEATH_DIAG native_control={:?} death={death:?} training_pending={}",
            runtime.player_control_facts(actor, admitted.game_session_id),
            states.has_training_to_flush(&runtime, actor, admitted.game_session_id));
        death
    };
    eprintln!("OFFLINE_DEATH_DIAG stop_timed={:?}", tokio::time::timeout(Duration::from_secs(5),
        owner.stop_timed_actor(actor, admitted.game_session_id)).await);
    eprintln!("OFFLINE_DEATH_DIAG save_proficiency={:?}", tokio::time::timeout(Duration::from_secs(5),
        owner.save_weapon_proficiency(actor, admitted.game_session_id, true)).await);
    eprintln!("OFFLINE_DEATH_DIAG save_resting={:?}", tokio::time::timeout(Duration::from_secs(5),
        owner.save_resting_stamina(actor, admitted.game_session_id)).await);
    eprintln!("OFFLINE_DEATH_DIAG save_hunting={:?}", tokio::time::timeout(Duration::from_secs(5),
        owner.save_hunting_stamina(actor, admitted.game_session_id)).await);
    eprintln!("OFFLINE_DEATH_DIAG training_before_save={:?}", tokio::time::timeout(Duration::from_secs(5),
        owner.training_before_save(actor, admitted.game_session_id, true)).await);
    eprintln!("OFFLINE_DEATH_DIAG save_training={:?}", tokio::time::timeout(Duration::from_secs(5),
        owner.save_spell_training(actor, admitted.game_session_id, true)).await);
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let checkpoint = states.get(&runtime, actor, admitted.game_session_id)
            .and_then(|state| state.pending_training_checkpoint());
        eprintln!("OFFLINE_DEATH_DIAG checkpoint={checkpoint:?}");
    }
    if let Some(death) = death {
        let settled = tokio::time::timeout(Duration::from_secs(10),
            owner.settle_death(actor, admitted.game_session_id, death, false)).await;
        eprintln!("OFFLINE_DEATH_DIAG settled={settled:?}");
        if let Some(fence) = admitted.item_fence {
            let placement = crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone())
                .read_death_return_placement(owner.character, owner.holder, fence, *death.occurrence.as_bytes()).await;
            eprintln!("OFFLINE_DEATH_DIAG death_placement={placement:?}");
        }
    }
}
