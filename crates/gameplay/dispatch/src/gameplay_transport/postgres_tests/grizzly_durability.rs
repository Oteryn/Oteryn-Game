//! W15c owner cases on mounted PostgreSQL. NPC/kill/portal ingress has separate
//! dispatch cases; these prove the locked transitions, replay and durable obligations.

use super::fixture::{Harness, debug, fence, id, run};
use super::TestResult;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{CharacterProgressionError, ExperienceRewardOccurrence};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::quest_state::grizzly::GrizzlyPremiumGate;
use crate::durability::quest_state::{
    CommittedQuestTransition, QuestCause, QuestStateCopy, QuestTransitionOutcome,
};
use crate::durability::quest_state::grizzly_boss::{GrizzlyBossEntryContext, GrizzlyBossEntryState};
use crate::durability::DurabilityError;
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration};
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestStateCatalogue, grizzly_adams as quest};
use quest::GrizzlyAction as Action;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

#[path = "grizzly_durability/compatibility.rs"]
mod compatibility;
#[path = "grizzly_durability/party.rs"]
mod party;
#[path = "grizzly_durability/rewards.rs"]
mod rewards;
#[path = "grizzly_durability/progression.rs"]
mod progression;

/// An owner-boundary fixture, like the existing CharmFacts double: verify that the
/// writer supplies the locked Character's account in the same live transaction.
/// Production Premium proof is covered by the NPC dispatcher case.
struct PremiumProbe {
    allowed: bool,
    calls: AtomicUsize,
}
impl PremiumProbe {
    fn new(allowed: bool) -> Self { Self { allowed, calls: AtomicUsize::new(0) } }
}
impl GrizzlyPremiumGate for PremiumProbe {
    async fn task_permitted(
        &self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        assert_eq!(account, id(40));
        sqlx::query("SELECT 1").execute(&mut **tx).await?;
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.allowed)
    }
}

fn catalogue() -> TestResult<Arc<QuestStateCatalogue>> {
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    Ok(Arc::new(lowered.catalogue().clone()))
}
fn command(n: u64) -> TestResult<QuestCause> {
    Ok(QuestCause::Command(CommandRef::new(fence(1)?.game_session_id,
        CommandId::new(n).map_err(debug)?)))
}
fn step(n: u64) -> TestResult<QuestCause> {
    let QuestCause::Command(command) = command(n)? else { return Err("command".into()); };
    Ok(QuestCause::Step(command))
}
fn death(n: u8) -> TestResult<QuestCause> {
    Ok(QuestCause::CreatureDeath(ExperienceRewardOccurrence::from_bytes(id(n)).map_err(debug)?))
}
fn solo_source() -> crate::foundation::PartySnapshot {
    crate::foundation::PartySnapshot {
        world: id(42), character: id(41), party_id: None, revision: None,
        leader: None, shared_xp_enabled: false, members: Vec::new(), observed_database_micros: 0,
    }
}
fn entry_context(boss_id: u8) -> TestResult<GrizzlyBossEntryContext> {
    let boss = quest::boss(boss_id).ok_or("boss")?;
    Ok(GrizzlyBossEntryContext {
        boss_id,
        exit: crate::foundation::MovementLocalPosition {
            x: i32::from(boss.portal.x), y: i32::from(boss.portal.y), floor: i16::from(boss.portal.z),
        },
        world: crate::durability::quest_world_callback::QuestWorldContext {
            map_revision: format!("sha256:{}", "07".repeat(32)),
            content_revision: format!("sha256:{}", "08".repeat(32)), placement_context: vec![6; 32],
            reset_epoch: 0, actor_placement: id(91), movement_revision: 1,
        },
    })
}
async fn state(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>) -> TestResult<QuestStateCopy> {
    h.root.read_character_quest_state(authority, fence(1)?.character_id).await.map_err(debug)
}
async fn value(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>, storage: u32) -> TestResult<i64> {
    Ok(quest::storage_value(state(h, authority).await?.tracks(), storage))
}
fn committed(result: QuestTransitionOutcome) -> TestResult<CommittedQuestTransition> {
    if let QuestTransitionOutcome::Committed(receipt) = result { Ok(receipt) }
    else { Err(format!("expected committed action, got {result:?}").into()) }
}
async fn apply(
    h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
    sequencer: &CharacterRevisionSequencer, catalogue: &Arc<QuestStateCatalogue>,
    cause: QuestCause, action: Action, gate: &PremiumProbe,
) -> TestResult<QuestTransitionOutcome> {
    let label = format!("{action:?} cause={cause:?}");
    match action {
        Action::EnterBoss(boss) => {
            let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence(1)?.runtime_scope).await.map_err(|error| format!("{label}: {error:?}"))?;
            sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_boss_entry(
                &h.root, authority, &h.node, fence(1)?, cause, Arc::clone(catalogue),
                &permit, entry_context(boss)?).await.map_err(|error| format!("{label}: {error:?}").into())
        }
        Action::KillCreature { .. } | Action::BossKill { .. } => {
            sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
                &h.root, authority, &h.node, fence(1)?, cause, action,
                Arc::clone(catalogue), solo_source()).await.map_err(|error| format!("{label}: {error:?}").into())
        }
        _ => sequencer.acquire(fence(1)?.character_id).await
            .commit_grizzly_transition_with_progression(&h.root, authority, &h.node, fence(1)?, cause,
                action, Arc::clone(catalogue), gate, Some(&rewards::policy())).await.map_err(|error| format!("{label}: {error:?}").into()),
    }
}

async fn boss_death(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>, boss_id: u8)
    -> TestResult<Action> {
    Ok(Action::BossKill { boss_id, observed_revision:
        state(h, authority).await?.character_revision().ok_or("boss death revision")?.get() })
}

async fn close_visit(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let entry = h.root.pending_grizzly_boss_entry(authority, fence(1)?.character_id)
        .await.map_err(debug)?.ok_or("pending visit")?;
    assert!(h.root.close_grizzly_boss_entry(authority, &h.node, fence(1)?, entry.cause,
        return_position(&entry)).await.map_err(|error| format!("close boss={} cause={:?}: {error:?}", entry.boss_id, entry.cause))?);
    Ok(())
}

fn return_position(entry: &crate::durability::quest_state::grizzly_boss::GrizzlyBossEntry)
    -> crate::durability::character_last_position::FinalLastPosition {
    use crate::durability::character_last_position::{FinalLastPosition, LastPositionCell, LastPositionRecord};
    FinalLastPosition {
        write_sequence: entry.cause.command_id().get(),
        record: LastPositionRecord {
            world_id: entry.world_id,
            cell: LastPositionCell { x: entry.exit.x, y: entry.exit.y, floor: entry.exit.floor },
            coordinate_frame_digest: [6; 32], map_revision_digest: [7; 32], server_artifact_digest: [8; 32],
            vitals: None, rest: None,
        },
    }
}

/// Only fixture predecessor facts use administrator privileges. Every action and
/// reward assertion below reaches the runtime role's real writer and SQL guards.
async fn seed(h: &Harness, level: Option<u32>, tracks: &[(u32, i64)]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    if let Some(level) = level {
        sqlx::query("UPDATE game_character_progression_state SET level=$2,total_experience=0 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(i64::from(level)).execute(&mut *tx).await?;
    }
    for (storage, value) in tracks {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(id(41).as_slice()).bind(quest::storage_track(*storage)).bind(quest::QUEST)
            .bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[test]
fn grizzly_durable_tasks_refusals_replay_restart_and_xp_relog() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(|| task_scenario().map_err(|error| error.to_string()))?.join()
        .map_err(|_| "Grizzly task owner PostgreSQL thread panicked")??;
    Ok(())
}

fn task_scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "grizzly_task_owner", true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let catalogue = catalogue()?;
        let yes = PremiumProbe::new(true);
        let no = PremiumProbe::new(false);
        seed(&h, Some(6), &[]).await?;

        let join = committed(apply(&h, &authority, &sequencer, &catalogue,
            command(1)?, Action::Join, &no).await?)?;
        assert_eq!(join.changes.len(), 4);
        assert_eq!(value(&h, &authority, quest::JOIN_STORAGE).await?, 1);
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(1)?, Action::Join, &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(join.clone()));
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(2)?, Action::Join, &no).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));

        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(3)?, Action::StartTask(1), &no).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
        // Upgrading Premium cannot reinterpret a refused command; a new command can act.
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(3)?, Action::StartTask(1), &yes).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
        assert_eq!(yes.calls.load(Ordering::Relaxed), 0);
        let start = committed(apply(&h, &authority, &sequencer, &catalogue,
            command(4)?, Action::StartTask(1), &yes).await?)?;
        assert_eq!(yes.calls.load(Ordering::Relaxed), 1);
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(4)?, Action::StartTask(1), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(start));
        let observed_revision = state(&h, &authority).await?.character_revision().ok_or("snapshot revision")?.get();
        let kill = Action::KillCreature { task_id: 1, creature: "crocodile".into(), observed_revision };
        let first_kill = committed(apply(&h, &authority, &sequencer, &catalogue,
            death(80)?, kill.clone(), &no).await?)?;
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(80)?, kill.clone(), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(first_kill));
        assert_eq!(value(&h, &authority, quest::KILL_COUNTER_BASE + 1).await?, 1);
        let conflict = sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
            &h.root, &authority, &h.node, fence(1)?, death(80)?,
            Action::KillCreature { task_id: 1, creature: "rat".into(), observed_revision },
            Arc::clone(&catalogue), solo_source()).await;
        assert!(matches!(conflict, Err(CharacterProgressionError::ConflictingOccurrence)), "{conflict:?}");
        let mut stale = fence(1)?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let rejected = sequencer.acquire(stale.character_id).await.commit_grizzly_kill(
            &h.root, &authority, &h.node, stale, death(81)?, kill.clone(), Arc::clone(&catalogue), solo_source()).await;
        assert!(matches!(rejected, Err(CharacterProgressionError::AuthorityRejected)), "{rejected:?}");
        assert_eq!(value(&h, &authority, quest::KILL_COUNTER_BASE + 1).await?, 1);

        let cancel = committed(apply(&h, &authority, &sequencer, &catalogue,
            command(5)?, Action::CancelTask(1), &no).await?)?;
        assert_eq!(value(&h, &authority, quest::QUEST_STORAGE_BASE + 1).await?, -1);
        committed(apply(&h, &authority, &sequencer, &catalogue, command(6)?, Action::StartTask(1), &yes).await?)?;
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(5)?, Action::CancelTask(1), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(cancel));
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(82)?, kill, &no).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch), "death from the old task run");
        assert_eq!(value(&h, &authority, quest::KILL_COUNTER_BASE + 1).await?, 0);
        seed(&h, None, &[(quest::KILL_COUNTER_BASE + 1, 299)]).await?;
        let observed_revision = state(&h, &authority).await?.character_revision().ok_or("revision")?.get();
        committed(apply(&h, &authority, &sequencer, &catalogue, death(83)?,
            Action::KillCreature { task_id: 1, creature: "crocodile".into(), observed_revision }, &no).await?)?;
        let before_completion = boss_death(&h, &authority, 1).await?;
        let completed = committed(apply(&h, &authority, &sequencer, &catalogue,
            command(7)?, Action::CompleteTask(1), &no).await?)?;
        assert_eq!(completed.experience.ok_or("XP obligation")?.amount, 800);
        assert_eq!(value(&h, &authority, quest::POINTS_STORAGE).await?, 1);
        assert_eq!(value(&h, &authority, quest::REPEAT_STORAGE_BASE + 1).await?, 1);
        assert_eq!(value(&h, &authority, quest::boss(1).ok_or("boss")?.storage).await?, 1);
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(7)?, Action::CompleteTask(1), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(completed));
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(8)?, Action::CompleteTask(1), &no).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
        for _ in 0..2 {
            assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(84)?, before_completion.clone(), &no).await?,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch), "completion reset excludes an earlier queued boss death");
        }
        assert_eq!(value(&h, &authority, quest::boss(1).ok_or("boss")?.kill_storage).await?, 0);
        seed(&h, None, &[(quest::POINTS_STORAGE, 10)]).await?;
        let promotion = committed(apply(&h, &authority, &sequencer, &catalogue,
            command(9)?, Action::Promote, &no).await?)?;
        assert_eq!(value(&h, &authority, quest::PROMOTION_STORAGE).await?, 0);
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(9)?, Action::Promote, &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(promotion));

        // Rebuild the process-local sequencer and state copy: admission's existing XP
        // owner consumes the durable obligation, independent of the earlier response.
        let rebuilt = CharacterRevisionSequencer::new();
        let mut copy = state(&h, &authority).await?;
        assert_eq!(copy.xp_obligations().len(), 1);
        assert!(!crate::durability::quest_state::request_pending_quest_experience(
            &rebuilt, &h.root, &authority, &h.node, fence(1)?, &rewards::policy(), &mut copy).await);
        assert!(copy.xp_obligations().is_empty());
        let (xp, receipts, pending): (i64, i64, i64) = sqlx::query_as(
            "SELECT total_experience,(SELECT count(*) FROM game_character_xp_receipts),(SELECT count(*) FROM game_character_quest_xp_obligations) FROM game_character_progression_state")
            .fetch_one(&h.pool).await?;
        assert_eq!((xp, receipts, pending), (800, 1, 0));
        let replay = h.root.reconcile_character_grizzly_transition(&authority, fence(1)?.character_id,
            command(7)?, Action::CompleteTask(1)).await.map_err(debug)?.ok_or("historical completion")?;
        assert!(matches!(replay, QuestTransitionOutcome::AlreadyCommitted(receipt) if receipt.experience.is_none()));
        assert_eq!(h.root.reconcile_character_grizzly_transition(&authority, fence(1)?.character_id,
            command(3)?, Action::StartTask(1)).await.map_err(debug)?,
            Some(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
        eprintln!("GRIZZLY_TASK_OWNER POSTGRES_CASE_COMPLETED");
        drop(authority); drop(seal); h.cleanup().await
    })
}

#[test]
fn grizzly_durable_boss_selection_entry_report_and_special_tasks() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(|| boss_scenario().map_err(|error| error.to_string()))?.join()
        .map_err(|_| "Grizzly boss owner PostgreSQL thread panicked")??;
    Ok(())
}

fn boss_scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "grizzly_boss_owner", true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let catalogue = catalogue()?;
        let no = PremiumProbe::new(false);
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        assert!(h.root.record_content_activation(&crate::durability::content_activation::ContentActivationRequest {
            world_id: fence(1)?.runtime_scope.world_id(),
            channel_id: crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            activation_sequence: 1, previous_sequence: None, server_artifact_digest: [8; 32],
            client_artifact_digest: [7; 32], frame_binding_digest: [6; 32],
        }).await.map_err(debug)?);
        seed(&h, Some(80), &[]).await?;
        committed(apply(&h, &authority, &sequencer, &catalogue, command(1)?, Action::Join, &no).await?)?;
        seed(&h, None, &[(quest::BOSS_POINTS_STORAGE, 1), (quest::POINTS_STORAGE, 70)]).await?;
        let before_selection = boss_death(&h, &authority, 1).await?;
        let selected = committed(apply(&h, &authority, &sequencer, &catalogue,
            command(2)?, Action::SelectBoss(1), &no).await?)?;
        assert_eq!(value(&h, &authority, quest::BOSS_POINTS_STORAGE).await?, 1, "first outgrown bracket is free");
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(2)?, Action::SelectBoss(1), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(selected));
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(3)?, Action::SelectBoss(1), &no).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
        for _ in 0..2 {
            assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(88)?, before_selection.clone(), &no).await?,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch), "selection reset excludes an earlier queued boss death");
        }
        let entered = committed(apply(&h, &authority, &sequencer, &catalogue,
            step(4)?, Action::EnterBoss(1), &no).await?)?;
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, step(4)?, Action::EnterBoss(1), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(entered));
        let visit = h.root.pending_grizzly_boss_entry(&authority, fence(1)?.character_id).await.map_err(debug)?.ok_or("visit")?;
        assert_eq!(visit.deadline_unix_ms - visit.entered_at_unix_ms, 600000);
        assert_eq!(visit.state, GrizzlyBossEntryState::Active);
        close_visit(&h, &authority).await?;
        // Returning without the kill rearms the entitlement without charging a point.
        committed(apply(&h, &authority, &sequencer, &catalogue, command(5)?, Action::ReportBoss(1), &no).await?)?;
        committed(apply(&h, &authority, &sequencer, &catalogue, step(6)?, Action::EnterBoss(1), &no).await?)?;
        let kill_action = boss_death(&h, &authority, 1).await?;
        let kill = committed(apply(&h, &authority, &sequencer, &catalogue,
            death(90)?, kill_action.clone(), &no).await?)?;
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(90)?, kill_action.clone(), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(kill.clone()));
        close_visit(&h, &authority).await?;
        let before_report = boss_death(&h, &authority, 1).await?;
        let report = committed(apply(&h, &authority, &sequencer, &catalogue,
            command(7)?, Action::ReportBoss(1), &no).await?)?;
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(7)?, Action::ReportBoss(1), &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(report));
        let boss = quest::boss(1).ok_or("boss")?;
        assert_eq!(value(&h, &authority, boss.storage).await?, 3);
        for _ in 0..2 {
            assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(91)?, before_report.clone(), &no).await?,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch), "report reset excludes an earlier queued boss death");
        }
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(90)?, kill_action, &no).await?,
            QuestTransitionOutcome::AlreadyCommitted(kill), "a retained kill receipt replays before the reset guard");
        assert_eq!(value(&h, &authority, boss.kill_storage).await?, 0);
        let achievement_key = "oteryn:achievement/blood_red_snapper";
        let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1")
            .bind(achievement_key).fetch_one(&h.pool).await?;
        assert_eq!(achievements, 1);
        let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key=$1 AND source_kind='oteryn:grizzly-adams'")
            .bind(achievement_key).fetch_one(&h.pool).await?;
        assert_eq!(grants, 1, "report replay preserves one canonical native achievement grant");
        let event_bytes: i32 = sqlx::query_scalar("SELECT octet_length(source_event_id) FROM game_account_achievement_grant_requests WHERE achievement_key=$1 AND source_kind='oteryn:grizzly-adams'")
            .bind(achievement_key).fetch_one(&h.pool).await?;
        assert_eq!(event_bytes, 32);
        committed(apply(&h, &authority, &sequencer, &catalogue, command(8)?, Action::SelectBoss(1), &no).await?)?;
        assert_eq!(value(&h, &authority, quest::BOSS_POINTS_STORAGE).await?, 0, "repeat spends one point");
        // Q17: a retained menu cannot spend the last point again or use the free
        // outgrown assignment after its original positive-balance authority expires.
        for (command_id, boss_id) in [(80, 10), (81, 2)] {
            for _ in 0..2 {
                assert_eq!(apply(&h, &authority, &sequencer, &catalogue,
                    command(command_id)?, Action::SelectBoss(boss_id), &no).await?,
                    QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
            }
            assert_eq!(value(&h, &authority, quest::BOSS_POINTS_STORAGE).await?, 0);
            assert_eq!(value(&h, &authority, quest::boss(boss_id).ok_or("boss")?.storage).await?, -1);
        }
        // Two genuine deaths from one capture revision both count; the first kill's
        // counter-track revision must not make the second look like a reset.
        let same_epoch = boss_death(&h, &authority, 1).await?;
        for occurrence in [92, 93] {
            committed(apply(&h, &authority, &sequencer, &catalogue, death(occurrence)?, same_epoch.clone(), &no).await?)?;
        }
        assert_eq!(value(&h, &authority, boss.kill_storage).await?, 2);
        for (id, base) in [(29, 20), (30, 30)] {
            let before_special = boss_death(&h, &authority, id).await?;
            let opened = committed(apply(&h, &authority, &sequencer, &catalogue,
                command(base)?, Action::StartSpecialTask, &no).await?)?;
            assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(base)?, Action::StartSpecialTask, &no).await?,
                QuestTransitionOutcome::AlreadyCommitted(opened));
            for _ in 0..2 {
                assert_eq!(apply(&h, &authority, &sequencer, &catalogue, death(id + 150)?, before_special.clone(), &no).await?,
                    QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch), "special-task reset excludes an earlier queued boss death");
            }
            committed(apply(&h, &authority, &sequencer, &catalogue, step(base + 1)?, Action::EnterBoss(id), &no).await?)?;
            let special_kill = boss_death(&h, &authority, id).await?;
            committed(apply(&h, &authority, &sequencer, &catalogue, death(id + 100)?, special_kill, &no).await?)?;
            close_visit(&h, &authority).await?;
            committed(apply(&h, &authority, &sequencer, &catalogue, command(base + 2)?, Action::StartSpecialTask, &no).await?)?;
            assert_eq!(value(&h, &authority, quest::boss(id).ok_or("special boss")?.storage).await?, 3);
        }
        assert_eq!(apply(&h, &authority, &sequencer, &catalogue, command(40)?, Action::StartSpecialTask, &no).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
        // A live room visit and its deadline are durable. Cleanup is Channel-owned,
        // so terminal/offline Character sessions cannot strand its occupied-room key.
        committed(apply(&h, &authority, &sequencer, &catalogue, step(50)?, Action::EnterBoss(1), &no).await?)?;
        let entry = h.root.pending_grizzly_boss_entry(&authority, fence(1)?.character_id)
            .await.map_err(debug)?.ok_or("room visit")?;
        let immutable = sqlx::query("UPDATE game_character_grizzly_boss_entries SET entered_at=entered_at+1,deadline_at=deadline_at+1 WHERE state='ACTIVE'")
            .execute(&h.pool).await;
        assert!(immutable.is_err(), "a relog cannot restart the ten-minute timer");
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_grizzly_boss_entries SET entered_at=entered_at-600001,deadline_at=deadline_at-600001 WHERE state='ACTIVE'")
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(50).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let wrong_owner = h.root.due_grizzly_boss_entries(&authority, &h.node, entry.world_id,
            entry.channel_id, crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?).await;
        assert!(matches!(wrong_owner, Err(CharacterProgressionError::AuthorityRejected)), "{wrong_owner:?}");
        let due = h.root.due_grizzly_boss_entries(&authority, &h.node, entry.world_id,
            entry.channel_id, entry.scope_generation).await.map_err(debug)?;
        assert_eq!(due.len(), 1);
        assert!(due[0].expired);
        assert!(h.root.expire_grizzly_boss_entry(&authority, &h.node, entry.world_id,
            entry.channel_id, entry.scope_generation, &due[0]).await.map_err(debug)?);
        assert!(h.root.due_grizzly_boss_entries(&authority, &h.node, entry.world_id,
            entry.channel_id, entry.scope_generation).await.map_err(debug)?.is_empty());
        let pending = h.root.pending_grizzly_boss_entry(&authority, fence(1)?.character_id)
            .await.map_err(debug)?.ok_or("offline return obligation")?;
        assert_eq!(pending.state, GrizzlyBossEntryState::ReturnPending);
        assert_eq!(pending.exit, entry.exit);
        assert_eq!(value(&h, &authority, boss.storage).await?, 2, "timeout does not reset donor storage");
        let closed = h.root.close_grizzly_boss_entry(&authority, &h.node, fence(1)?, pending.cause,
            return_position(&pending)).await;
        assert!(matches!(closed, Err(CharacterProgressionError::AuthorityRejected)), "{closed:?}");
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(50).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        close_visit(&h, &authority).await?;
        assert!(h.root.pending_grizzly_boss_entry(&authority, fence(1)?.character_id).await.map_err(debug)?.is_none());
        // Lost response followed by rebuilt admission cannot resurrect the expired
        // room: its CLOSED record and the outside last position committed together.
        let stored = h.root.read_character_last_position(&authority, &h.node, fence(1)?).await.map_err(debug)?.ok_or("durable returned position")?;
        assert_eq!(stored.record.cell, return_position(&pending).record.cell);
        assert!(!h.root.close_grizzly_boss_entry(&authority, &h.node, fence(1)?, pending.cause,
            return_position(&pending)).await.map_err(debug)?);
        eprintln!("GRIZZLY_BOSS_OWNER POSTGRES_CASE_COMPLETED");
        drop(authority); drop(seal); h.cleanup().await
    })
}
