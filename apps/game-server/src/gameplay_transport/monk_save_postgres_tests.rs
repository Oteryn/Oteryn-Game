#![allow(clippy::expect_used)]

#[path = "../../tests/support/character_revision_sequencer_postgres_cases.rs"]
mod fixture;

#[path = "monk_save_fresh_fixture.rs"]
mod fresh_fixture;

#[path = "kill_reward_postgres_e2e_tests.rs"]
mod kill_reward_e2e;

#[path = "logout_postgres_tests.rs"]
mod logout_e2e;

#[path = "map_bundle_domain_postgres_tests.rs"]
mod map_bundle_domain;

#[path = "weather_wire_postgres_tests.rs"]
mod weather_wire;

use super::{ComposedFreshAdmission, MonkSave};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, GraceExpiryResult, SessionContinuity,
};
use crate::gameplay_transport::fresh_evidence::FreshEvidenceSource;
use fixture::{Harness, debug, fence, id, run};
use std::cell::Cell;
use std::time::Duration;
use tokio::sync::Mutex;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
thread_local! {
    static HOST_NOW: Cell<Option<i64>> = const { Cell::new(None) };
    static SAVES: Cell<usize> = const { Cell::new(0) };
    static NOT_EXPIRED: Cell<usize> = const { Cell::new(0) };
}
pub(crate) fn host_now() -> Option<i64> {
    HOST_NOW.get()
}
pub(crate) fn record_save_attempt() {
    SAVES.set(SAVES.get() + 1);
}
pub(crate) fn record_not_expired() {
    NOT_EXPIRED.set(NOT_EXPIRED.get() + 1);
}
fn reset() {
    HOST_NOW.set(None);
    SAVES.set(0);
    NOT_EXPIRED.set(0);
}

async fn prepare_current_session(harness: &Harness) -> TestResult {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
    use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    // The new harness starts with no admission claims/session/history.
    // Complete owning evidence is published and committed through existing APIs.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let source = fresh_fixture::fresh::Source::new(now)?;
    let mut runtime_source = fresh_fixture::fresh::Source::new(now)?;
    runtime_source.rows.retain(|row| matches!(row.key,
        crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let runtime_publication =
        AdmissionAuthorityPublicationV1::prepare(&runtime_source, now).map_err(debug)?;
    harness
        .root
        .publish_runtime_readiness(&harness.node, &runtime_publication)
        .await
        .map_err(debug)?;
    let mut admission_source = fresh_fixture::fresh::Source::new(now)?;
    admission_source.rows.retain(|row| !matches!(row.key,
        crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication =
        AdmissionAuthorityPublicationV1::prepare(&admission_source, now).map_err(debug)?;
    let guards = AdmissionGuardStore::from_root(harness.root.clone());
    guards
        .publish(&publication)
        .await
        .map_err(|e| format!("fresh fixture publication: {e:?}"))?;
    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let request = source.request()?;
    assert!(matches!(
        store
            .commit(&request)
            .await
            .map_err(|e| format!("fresh fixture commit: {e:?}"))?,
        FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    let (current, _) = store
        .current_session_at(GameSessionId::decode(&id(50)).map_err(debug)?)
        .await?;
    assert_eq!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
    );
    assert_eq!(current.current_character_lease().generation(), 2);
    Ok(())
}

struct Dependencies {
    runtime: Mutex<ChannelRuntimeV1>,
    room: crate::content::QualifiedNativeEntryRoom,
    door: Mutex<crate::world_runtime::LocalObjectRuntime>,
    evidence: FreshEvidenceSource,
    spells: crate::spell::SpellBook,
    achievements: crate::achievement_catalogue::AchievementCatalogue,
    charms: crate::content::charm_source::CanonicalCharmCatalogue,
    admitted: AdmittedSession,
    actor: ExactActorRef,
}
impl Dependencies {
    fn new() -> TestResult<Self> {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        let expected = fence(1)?;
        let world = expected.runtime_scope.world_id();
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world),
        )
        .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
        let content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
            .map_err(debug)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            expected.runtime_scope,
            expected.scope_ownership_generation,
            content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            expected.runtime_scope,
            expected.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![CertificateDer::from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![CertificateDer::from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            PrivateKeyDer::try_from(
                include_bytes!("../../tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        Ok(Self {
            runtime: Mutex::new(runtime),
            room,
            door: Mutex::new(door),
            evidence: FreshEvidenceSource::new(descriptor),
            spells: crate::spell::cast::v1_spell_book().map_err(debug)?,
            achievements: crate::achievement_catalogue::AchievementCatalogue::embedded()
                .map_err(debug)?,
            charms: crate::content::charm_source::CanonicalCharmCatalogue::embedded()
                .map_err(debug)?,
            actor,
            admitted: AdmittedSession {
                game_session_id: session,
                world_id: world,
                channel_id: channel,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: Some(ControllerBinding {
                    transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                        .map_err(debug)?,
                    account_id: id(40),
                }),
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            },
        })
    }
    fn owner<'a, 'f, 's>(
        &'a self,
        harness: &'a Harness,
        authority: &'a ReconciledCharacterAuthority<'f, 's>,
    ) -> ComposedFreshAdmission<'a, 'f, 's> {
        ComposedFreshAdmission {
            root: &harness.root,
            character: authority,
            holder: &harness.node,
            evidence: &self.evidence,
            world_id: self.admitted.world_id,
            channel_id: self.admitted.channel_id,
            // Gameplay-only fixture: it never admits through this owner, so the ADMIT0 binding
            // declares nothing and only has to be a valid Channel boot binding.
            required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                crate::foundation::RuntimeScopeRefV1::channel(self.admitted.world_id, self.admitted.channel_id),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .expect("an empty declared set binds"),
            runtime: &self.runtime,
            movement_cells: self.room.movement_cells(),
            door: &self.door,
            chest: self.room.door(),
            spells: &self.spells,
            active_generation: None,
            premium_coordinator: None,
            rooms: crate::gameplay_transport::SeamRooms::new(None, None),
            achievements: &self.achievements,
            imported_charms: &self.charms,
            spell_states: Mutex::default(),
            world_gate: std::sync::Mutex::default(),
            attack: Mutex::default(),
            clock_origin: std::time::Instant::now(),
            lost: std::sync::Mutex::default(),
            revision_sequencer:
                crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
            quest_catalogue: None,
            quest_sessions: std::sync::Mutex::default(),
            chat: crate::gameplay_transport::chat_intent::ChatRuntime::default(),
            npc_conversations: Default::default(),
            wheel_ruleset: None,
            wheel_sessions: std::sync::Mutex::default(),
            progression_sessions: std::sync::Mutex::default(),
            last_positions: crate::gameplay_transport::last_position::LastPositions::default(),
            input_gate: Default::default(),
            premium: crate::premium::refresh::PremiumRefresher::new(
                Default::default(),
                harness.root.clone(),
                None,
            ),
            premium_sessions: std::sync::Mutex::default(),
            fence_holders: Default::default(),
            spell_lane: crate::durability::spell_owner_commit::SpellLane::new(
                self.admitted.world_id,
                self.admitted.channel_id,
            ),
            loop_ground: Default::default(),
        }
    }
    async fn initialize(&self, owner: &ComposedFreshAdmission<'_, '_, '_>) {
        let runtime = self.runtime.lock().await;
        let mut states = owner.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    self.actor,
                    self.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::Monk,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (3, 7_000_000),
                    owner.owner_now(),
                )
                .is_some()
        );
    }
    async fn loss(&self, harness: &Harness, horizon: i64) -> TestResult<i64> {
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&harness.pool)
                .await?;
        let deadline = now + horizon;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1, current_transport_ref=NULL, control_loss_epoch=1, predecessor_generation=current_generation, original_grace_deadline=$1 WHERE game_session_id=encode($2,'hex')::uuid")
            .bind(deadline).bind(id(50).as_slice()).execute(&harness.pool).await?;
        self.runtime
            .lock()
            .await
            .record_control_loss(
                self.actor,
                self.admitted.game_session_id,
                crate::foundation::ControlLossMark {
                    epoch: 1,
                    grace_deadline: deadline,
                },
            )
            .map_err(debug)?;
        Ok(now)
    }
}
async fn revision(h: &Harness) -> TestResult<u64> {
    let value: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    Ok(value.parse()?)
}
async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_monk_state_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}

#[test]
fn boundary_release_retries_not_expired_and_saves_new_remaining_serene() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "release_monk_retry", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        let now = dependencies.loss(&harness, 2).await?;
        // One invariant changes: the owner's host clock is faster than the independent
        // durable clock. Both mirrors retain the identical immutable grace deadline.
        HOST_NOW.set(Some(now + 60));
        let observation = async {
            tokio::time::timeout(Duration::from_secs(4), async {
                loop {
                    if NOT_EXPIRED.get() > 0 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await?;
            assert_eq!(revision(&harness).await?, 2);
            // Observe the committed intermediate save through the independent fixture
            // pool. The owner loop deliberately occupies the root's only ready holder,
            // so a concurrent semantic read would race with the behavior under test.
            let first: (i16, i64) = sqlx::query_as(
                "SELECT harmony, serene_forced_remaining_micros \
                 FROM game_character_progression_state",
            )
            .fetch_one(&harness.pool)
            .await?;
            Ok::<_, Box<dyn std::error::Error>>(first)
        };
        let (result, observed) = fixture::join_two(
            tokio::time::timeout(
                Duration::from_secs(8),
                owner.release_after_grace(dependencies.admitted),
            ),
            observation,
        )
        .await;
        let first = observed?;
        assert_eq!(result?, GraceExpiryResult::Released);
        assert!(NOT_EXPIRED.get() >= 1 && SAVES.get() >= 2);
        let final_state = harness
            .root
            .read_character_monk_state(&authority, fence(1)?.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(final_state.harmony(), 3);
        assert_eq!(first.0, 3);
        assert!(final_state.serene_forced_remaining_micros() < u64::try_from(first.1)?);
        assert_eq!(revision(&harness).await?, 3);
        assert_eq!(receipt_count(&harness).await?, 2);
        let (session, _) = FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(dependencies.admitted.game_session_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            session.session_state(),
            crate::foundation::GameSessionState::Terminal
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Polls `release` until `reached` holds. A release that ends first is returned as the error.
async fn drive_until<F: std::future::Future>(
    release: F,
    reached: impl Fn() -> bool,
) -> Result<(), F::Output> {
    use std::task::Poll;
    let mut release = std::pin::pin!(release);
    let mut probe = std::pin::pin!(async {
        while !reached() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    std::future::poll_fn(move |context| {
        if let Poll::Ready(output) = release.as_mut().poll(context) {
            return Poll::Ready(Err(output));
        }
        if probe.as_mut().poll(context).is_ready() {
            return Poll::Ready(Ok(()));
        }
        Poll::Pending
    })
    .await
}

#[test]
fn boundary_unknown_monk_save_keeps_lease_until_store_recovers() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "release_monk_unknown", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        dependencies.loss(&harness, -1).await?;
        // Only the own fixture's monk receipt relation becomes unavailable.
        sqlx::query("ALTER TABLE game_character_monk_state_receipts RENAME TO spell_fixture_unavailable_receipts").execute(&harness.pool).await?;
        // The release keeps the lease while the store is unavailable. A second monk save
        // attempt starts only after the first one ended `Unknown` and the release loop
        // retried instead of ending the lease, so the test waits for that event, not for a
        // wall-clock window. The deadline only bounds a hang.
        let kept = tokio::time::timeout(
            Duration::from_secs(60),
            drive_until(owner.release_after_grace(dependencies.admitted), || {
                SAVES.get() >= 2
            }),
        )
        .await?;
        assert!(
            kept.is_ok(),
            "the release must not end while the monk store is unavailable: {:?}",
            kept.err()
        );
        assert!(
            SAVES.get() >= 2,
            "the real release loop must retry the monk save boundary"
        );
        assert_eq!(revision(&harness).await?, 1);
        let (session, _) = FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(dependencies.admitted.game_session_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            session.session_state(),
            crate::foundation::GameSessionState::Reconnectable
        );
        sqlx::query("ALTER TABLE spell_fixture_unavailable_receipts RENAME TO game_character_monk_state_receipts").execute(&harness.pool).await?;
        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(8),
                owner.release_after_grace(dependencies.admitted)
            )
            .await?,
            GraceExpiryResult::Released
        );
        assert_eq!(revision(&harness).await?, 2);
        assert_eq!(receipt_count(&harness).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn boundary_terminal_session_fences_actor_save_without_mutation() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "release_monk_terminal", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        dependencies.loss(&harness, -1).await?;
        assert!(matches!(
            FreshAdmissionStore::from_root(harness.root.clone())
                .release_expired_loss(
                    dependencies.admitted.game_session_id,
                    &crate::gameplay_transport::canonical_uuid(&id(40))
                )
                .await
                .map_err(debug)?,
            crate::durability::fresh_admission::ExpiredLossReleaseV1::Released { .. }
        ));
        assert_eq!(
            owner
                .save_monk_state(&dependencies.admitted, dependencies.actor)
                .await,
            MonkSave::FencedOut
        );
        assert_eq!(revision(&harness).await?, 1);
        assert_eq!(receipt_count(&harness).await?, 0);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn boundary_removed_actor_cannot_save_values_from_its_old_slot() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "release_monk_removed", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        dependencies
            .runtime
            .lock()
            .await
            .remove_test_actor(dependencies.actor)
            .map_err(debug)?;
        assert_eq!(
            owner
                .save_monk_state(&dependencies.admitted, dependencies.actor)
                .await,
            MonkSave::NotApplicable
        );
        assert_eq!(revision(&harness).await?, 1);
        assert_eq!(receipt_count(&harness).await?, 0);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// OPS-SAVE-1: tracks the admitted session as the node does after a successful admission read.
fn track(owner: &ComposedFreshAdmission<'_, '_, '_>, dependencies: &Dependencies) -> TestResult {
    use crate::gameplay_transport::last_position::AdmissionPosition;
    assert!(owner.track_restored(
        dependencies.admitted.game_session_id,
        dependencies.actor,
        &AdmissionPosition::FirstLogin,
        fence(1)?,
    ));
    Ok(())
}

#[test]
fn shutdown_flush_saves_live_monk_values_and_skips_untracked_actors() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "flush_monk_live", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        // Untracked: nothing is written.
        assert_eq!(owner.flush_monk_states(&mut Default::default()).await, 0);
        assert_eq!(receipt_count(&harness).await?, 0);
        assert_eq!(revision(&harness).await?, 1);
        track(&owner, &dependencies)?;
        // The session stays live and fenced in; the flush is the only writer.
        assert_eq!(owner.flush_monk_states(&mut Default::default()).await, 0);
        let state = harness
            .root
            .read_character_monk_state(&authority, fence(1)?.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.harmony(), 3);
        assert!(state.serene_forced_remaining_micros() > 0);
        assert!(state.serene_forced_remaining_micros() <= 7_000_000);
        assert_eq!(revision(&harness).await?, 2);
        assert_eq!(receipt_count(&harness).await?, 1);
        let (session, _) = FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(dependencies.admitted.game_session_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            session.session_state(),
            crate::foundation::GameSessionState::Active
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn shutdown_flush_is_fenced_out_for_a_terminal_session_and_reports_an_unavailable_store()
-> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "flush_monk_fenced", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        track(&owner, &dependencies)?;
        // An unproven save is reported, so the flush is retried and never marked done.
        sqlx::query("ALTER TABLE game_character_monk_state_receipts RENAME TO spell_fixture_unavailable_receipts").execute(&harness.pool).await?;
        assert_eq!(owner.flush_monk_states(&mut Default::default()).await, 1);
        assert_eq!(revision(&harness).await?, 1);
        sqlx::query("ALTER TABLE spell_fixture_unavailable_receipts RENAME TO game_character_monk_state_receipts").execute(&harness.pool).await?;
        // A session that ended meanwhile is fenced out: nothing is written, nothing is retried.
        dependencies.loss(&harness, -1).await?;
        assert!(matches!(
            FreshAdmissionStore::from_root(harness.root.clone())
                .release_expired_loss(
                    dependencies.admitted.game_session_id,
                    &crate::gameplay_transport::canonical_uuid(&id(40))
                )
                .await
                .map_err(debug)?,
            crate::durability::fresh_admission::ExpiredLossReleaseV1::Released { .. }
        ));
        assert_eq!(owner.flush_monk_states(&mut Default::default()).await, 0);
        assert_eq!(revision(&harness).await?, 1);
        assert_eq!(receipt_count(&harness).await?, 0);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// DEATH-ML-LOSS-1 (CP D1440, A13 §4.6, SKILLS-0 §3.6) end to end on PostgreSQL through the
/// composed owner: a trained level 20 druid takes a real lethal creature hit, and
/// `respawn_after_death` force-saves its pending training, settles the death with the active
/// generation's training formula (the build loss lands in the death receipt under binding
/// version 2), places and consumes the respawn, and rebases the live actor onto the lowered
/// build, so its next training save commits on that build. The production generation supplies
/// the training formula and the progression; only its training content revision is the fixture
/// Character's interpretation (production pairs both as `s3b-content-1`).
mod death_ml_loss_composed {
    use super::fixture::{Harness, debug, id, run};
    use super::{Dependencies, TestResult, prepare_current_session};
    use crate::ability::creature_bite::CreatureBiteVitals;
    use crate::content::native_gameplay::NativeGameplayInput;
    use crate::content::{
        ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
        activate_native_entry_room_with_gameplay,
    };
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
        DurableBuildState, convert_vocation,
    };
    use crate::durability::character_progression::{
        CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceCommitOutcome,
        ExperienceRewardOccurrence,
    };
    use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
    use crate::foundation::{CharacterId, CharacterLease, GameSessionId, WorldId};
    use crate::gameplay_transport::ComposedFreshAdmission;
    use crate::gameplay_transport::actor_spell::TrainingSave;
    use crate::spell::cast::CharacterCastFacts;
    use crate::spell::mana_training::RetainedTrainingCheckpoint;
    use crate::spell::mana_training::hit_training::SkillFamily;
    use oteryn_simulation_determinism::ExactI64;

    /// The debug-build death settle chain is deeper than the default test stack (as in the
    /// kill-reward end-to-end case).
    const SCENARIO_STACK_BYTES: usize = 64 << 20;
    /// The production experience table's level 20 floor.
    const LEVEL_20_EXPERIENCE: u64 = 98_800;

    #[test]
    fn composed_owner_death_lowers_rebases_and_trains_on_postgres() -> TestResult {
        std::thread::Builder::new()
            .name("death-ml-loss-composed".into())
            .stack_size(SCENARIO_STACK_BYTES)
            .spawn(|| scenario(Case::Clean).map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "death ML loss composed thread panicked")?
            .map_err(Into::into)
    }

    /// #2193 Codex P1 4306765047: the death flush's retained checkpoint committed, but under a
    /// fence the session has since moved past, so every forced replay is fenced out. The live
    /// death applies the durable receipt, retiring the checkpoint, and respawns; the death is
    /// priced from the build holding the flushed try.
    #[test]
    fn composed_live_death_applies_a_committed_but_fenced_checkpoint_on_postgres() -> TestResult {
        std::thread::Builder::new()
            .name("death-ml-loss-fenced".into())
            .stack_size(SCENARIO_STACK_BYTES)
            .spawn(|| scenario(Case::CommittedFenced).map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "death ML loss fenced thread panicked")?
            .map_err(Into::into)
    }

    /// #2193 Codex P1 4307011952, 4307264455: the actor-end release's death flush replays a
    /// checkpoint the store refuses before any write (`AuthorityRejected`: its fence predates the
    /// session's moved lease), so the flushed try and the checkpoint stay only in memory. The
    /// release keeps the dead actor instead of committing the death from the older durable
    /// build, the proven-uncommitted checkpoint is re-fenced to the session's current authority,
    /// and the retried release flushes the try and settles the death priced from it.
    #[test]
    fn composed_release_refences_a_rejected_flush_then_settles_on_postgres() -> TestResult {
        std::thread::Builder::new()
            .name("death-ml-loss-rejected".into())
            .stack_size(SCENARIO_STACK_BYTES)
            .spawn(|| scenario(Case::ReleaseRejected).map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "death ML loss rejected thread panicked")?
            .map_err(Into::into)
    }

    /// #2193 Codex P1 4307264455: the live death's flush checkpoint is refused before any write
    /// under a fence the session's lease has moved past. The first cadence keeps the actor dead
    /// and re-fences the proven-uncommitted checkpoint to the current authority; the next one
    /// flushes the try, prices the death from it and respawns, so the actor is never stuck.
    #[test]
    fn composed_live_death_refences_a_rejected_flush_then_respawns_on_postgres() -> TestResult {
        std::thread::Builder::new()
            .name("death-ml-loss-live-rejected".into())
            .stack_size(SCENARIO_STACK_BYTES)
            .spawn(|| scenario(Case::LiveRejected).map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "death ML loss live rejected thread panicked")?
            .map_err(Into::into)
    }

    /// Which death flush a scenario composes.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Case {
        /// The flush saves the pending try under the current fence.
        Clean,
        /// The flush's checkpoint committed, but under a fence the session has since moved past.
        CommittedFenced,
        /// The flush's checkpoint is refused before any write, and the actor-end release settles.
        ReleaseRejected,
        /// The flush's checkpoint is refused before any write, and the live cadence respawns.
        LiveRejected,
    }

    /// The production manifest's generation, activated for `world`, with its training section
    /// interpreted under `content` (the Character root's content revision).
    fn production_generation(
        world: WorldId,
        content: &str,
    ) -> TestResult<ContentActivationController> {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/spells.manifest.json");
        let mut supplied = NativeGameplayInput::from_manifest(&manifest).map_err(debug)?;
        supplied.source_world = None;
        supplied
            .build_training
            .as_mut()
            .ok_or("the manifest pins build training")?
            .content_revision = content.into();
        let room = crate::content::qualify_selected_native_gameplay_room(world, &supplied)
            .map_err(debug)?;
        let issuance = NativeEntryActivationIssuance {
            world_id: world,
            activation_sequence: 1,
            server_artifact_digest: room.compiled().server_digest(),
            client_artifact_digest: room.compiled().client_digest(),
            frame_binding_digest: room.frame_binding().digest(),
        };
        let mut controller = ContentActivationController::new();
        activate_native_entry_room_with_gameplay(
            &mut controller,
            &NodeBootQuiescence::before_channel_runtime(),
            world,
            &issuance,
            &supplied,
        )
        .map_err(debug)?;
        Ok(controller)
    }

    async fn current_fence(
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        session: GameSessionId,
    ) -> TestResult<CurrentCharacterGameplayFence> {
        Ok(owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("terminal session")?)
    }

    /// The durable build row, or the death receipt's `before`/`after` columns, as a build.
    async fn build_row(h: &Harness, sql: &str) -> TestResult<DurableBuildState> {
        let row: (String, i32, i64, Vec<i32>, Vec<i64>) =
            sqlx::query_as(sqlx::AssertSqlSafe(sql.to_owned()))
                .bind(id(41).as_slice())
                .fetch_one(&h.pool)
                .await?;
        let mut skills = [(0, 0); 7];
        for (skill, (level, tries)) in skills.iter_mut().zip(row.3.iter().zip(&row.4)) {
            *skill = (u16::try_from(*level)?, u64::try_from(*tries)?);
        }
        DurableBuildState::new(
            row.0,
            (u16::try_from(row.1)?, u64::try_from(row.2)?),
            skills,
        )
        .map_err(debug)
    }

    fn columns(suffix: &str) -> String {
        let families = [
            "fist",
            "club",
            "sword",
            "axe",
            "distance",
            "shielding",
            "fishing",
        ];
        let levels: Vec<String> = families
            .iter()
            .map(|family| format!("{family}_level{suffix}"))
            .collect();
        let tries: Vec<String> = families
            .iter()
            .map(|family| format!("{family}_tries{suffix}"))
            .collect();
        format!(
            "vocation, magic_level{suffix}, mana_spent{suffix}, ARRAY[{}], ARRAY[{}]",
            levels.join(","),
            tries.join(",")
        )
    }

    /// The Character's death receipts.
    async fn death_receipts(h: &Harness) -> TestResult<i64> {
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM game_character_death_receipts \
             WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?)
    }

    /// Independent restatement of SKILLS-0 §3.6: sum the paid requirements and current progress,
    /// remove `floor(total * lost / before)` and re-level from the family floor.
    fn expected_loss(
        formula: &dyn BuildFormula,
        before: &DurableBuildState,
        experience_before: u64,
        experience_lost: u64,
    ) -> TestResult<DurableBuildState> {
        let vocation = before.vocation();
        let mut families = [before.magic(); 8];
        families[1..].copy_from_slice(&before.skills());
        for (family, (level, progress)) in families.iter_mut().enumerate() {
            let floor = if family == 0 { 0 } else { 10 };
            let mut total = *progress;
            for next in floor + 1..=*level {
                total += formula
                    .required(vocation, family, next)
                    .ok_or("paid requirement")?;
            }
            let removed = u64::try_from(
                u128::from(total) * u128::from(experience_lost) / u128::from(experience_before),
            )?;
            let mut remaining = total - removed;
            let mut relevelled = floor;
            while let Some(required) = formula.required(vocation, family, relevelled + 1) {
                if remaining < required {
                    break;
                }
                remaining -= required;
                relevelled += 1;
            }
            (*level, *progress) = (relevelled, remaining);
        }
        let mut skills = [(0, 0); 7];
        skills.copy_from_slice(&families[1..]);
        DurableBuildState::new(vocation, families[0], skills).map_err(debug)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one end-to-end scenario: setup, the lethal hit, the respawn and its checks"
    )]
    fn scenario(case: Case) -> TestResult {
        run(async move |admin| {
            let name = match case {
                Case::Clean => "death_ml_composed",
                Case::CommittedFenced => "death_ml_fenced",
                Case::ReleaseRejected => "death_ml_rejected",
                Case::LiveRejected => "death_ml_live_rejected",
            };
            let harness = Harness::create_with_admission_seed(admin, name, false, false).await?;
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
            let controller = production_generation(dependencies.admitted.world_id, "content-1")?;
            let active = controller.active().ok_or("activated generation")?;
            let formula = active
                .native_gameplay()
                .and_then(|native| native.training_formula())
                .ok_or("the generation's training formula")?;
            let table: &dyn BuildFormula = formula;
            let mut owner = dependencies.owner(&harness, &authority);
            owner.active_generation = Some(active);

            // ARCH-PROGRESSION-SOURCE-0 §1.5: the production admission step binds (and
            // initializes) the session's progression from the Character root's revisions.
            let mut admitted = dependencies.admitted;
            owner.admit_progression_session(&mut admitted).await;
            let progression = owner
                .progression_binding(session)
                .ok_or("the session binds the generation's progression")?;

            // Setup through the durable root under the session's current fences: the druid
            // choice, level 20 experience and a trained build (magic 3 and sword 14, each half-way
            // to its next level).
            let occurrence = |tag| BuildOccurrence::from_bytes(id(tag)).map_err(debug);
            let druid =
                convert_vocation(table, &DurableBuildState::default(), "druid").map_err(debug)?;
            let chosen = harness
                .root
                .commit_character_build(
                    &authority,
                    &harness.node,
                    current_fence(&owner, session).await?,
                    BuildChangeRequest {
                        occurrence: occurrence(90)?,
                        cause: BuildCause::VocationChoice,
                        before: DurableBuildState::default(),
                        after: druid.clone(),
                        pruned_stance: None,
                    },
                    table,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
            let award = harness
                .root
                .commit_character_experience(
                    &authority,
                    &harness.node,
                    current_fence(&owner, session).await?,
                    ExperienceAwardRequest {
                        occurrence: ExperienceRewardOccurrence::from_bytes(id(91))
                            .map_err(debug)?,
                        amount: ExactI64::new(i64::try_from(LEVEL_20_EXPERIENCE)?),
                        context: progression.context.clone(),
                        policy_revision: progression.policy_revision.clone(),
                        reward_revision: progression.reward_revision.clone(),
                        policy: progression.policy.clone(),
                    },
                )
                .await
                .map_err(debug)?;
            assert!(matches!(award, ExperienceCommitOutcome::Committed(_)));
            let half = |family, level| {
                table
                    .required("druid", family, level)
                    .map(|required| required / 2)
                    .ok_or("requirement")
            };
            let mut skills = druid.skills();
            skills[2] = (14, half(3, 15)?);
            let trained =
                DurableBuildState::new("druid", (3, half(0, 4)?), skills).map_err(debug)?;
            let BuildCommitOutcome::Committed(_) = harness
                .root
                .commit_character_build(
                    &authority,
                    &harness.node,
                    current_fence(&owner, session).await?,
                    BuildChangeRequest {
                        occurrence: occurrence(92)?,
                        cause: BuildCause::Training,
                        before: druid.clone(),
                        after: trained.clone(),
                        pruned_stance: None,
                    },
                    table,
                )
                .await
                .map_err(debug)?
            else {
                return Err("trained build replayed".into());
            };
            // The owner's revision writer starts after the setup writes, as after an admission.
            owner.revision_sequencer = CharacterRevisionSequencer::new();
            // The fence a checkpoint prepared before the session's lease moved retains.
            let stale = CurrentCharacterGameplayFence {
                character_lease_generation: 1,
                ..current_fence(&owner, session).await?
            };
            let mut retained_request = None;

            // The admitted actor, as first entry leaves it: placed, its Character lease bound,
            // its spell state loaded from the durable build with training enabled.
            {
                let mut runtime = dependencies.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        actor,
                        session,
                        CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                            .map_err(debug)?,
                    )
                    .map_err(debug)?;
                let mut states = owner.spell_states.lock().await;
                assert!(
                    states
                        .initialize(
                            &runtime,
                            actor,
                            session,
                            CharacterCastFacts {
                                level: 20,
                                magic_level: u32::from(trained.magic().0),
                                ..crate::gameplay_transport::actor_spell::tests::FACTS
                            },
                            (0, 0),
                            owner.owner_now(),
                        )
                        .is_some()
                );
                assert!(states.load_owned_build(&runtime, actor, session, &trained));
                let state = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("live actor")?;
                state
                    .enable_owned_training(&trained, formula)
                    .map_err(debug)?;
                // One sword try the cadence has not saved yet: the death flush must save it.
                state
                    .train_skill(SkillFamily::Sword, 1, formula, |_| 1)
                    .map_err(debug)?;
                assert!(state.has_training_to_flush());
            }

            // A real lethal creature hit records the death.
            let death = {
                let mut runtime = dependencies.runtime.lock().await;
                let mut states = owner.spell_states.lock().await;
                let now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                    owner.owner_now().get(),
                );
                let hit = states
                    .apply_creature_damage(&mut runtime, actor, session, 100_000, now)
                    .ok_or("the hit lands")?;
                assert_eq!(hit.damage.health_after, 0);
                states
                    .player_death(&runtime, actor, session)
                    .ok_or("the lethal hit records the death")?
            };
            assert!(owner.spell_states.lock().await.is_dead(actor));
            if case != Case::Clean {
                // The death flush's own checkpoint for the try, retained under the stale fence
                // (a retained checkpoint refuses creature hits, so it follows the death).
                let runtime = dependencies.runtime.lock().await;
                let mut states = owner.spell_states.lock().await;
                let state = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("dead actor")?;
                let prepared = state
                    .prepare_training_checkpoint(
                        owner.owner_now().get(),
                        formula,
                        occurrence(93)?,
                        true,
                    )
                    .map_err(debug)?
                    .ok_or("a checkpoint for the try")?;
                if case == Case::CommittedFenced {
                    retained_request = Some(prepared.request.clone());
                }
                state
                    .retain_training_checkpoint(RetainedTrainingCheckpoint {
                        fence: stale,
                        prepared,
                        formula_digest: formula.digest(),
                    })
                    .map_err(debug)?;
            }
            if let Some(request) = retained_request {
                // Its write committed before the post-write authority check saw the moved fence.
                let committed = harness
                    .root
                    .commit_character_build(
                        &authority,
                        &harness.node,
                        current_fence(&owner, session).await?,
                        request,
                        table,
                    )
                    .await
                    .map_err(debug)?;
                assert!(matches!(committed, BuildCommitOutcome::Committed(_)));
                owner.revision_sequencer = CharacterRevisionSequencer::new();
                // Every forced replay is fenced out and leaves the checkpoint retained.
                for _ in 0..2 {
                    assert_eq!(
                        owner.save_spell_training(actor, session, true).await,
                        TrainingSave::FencedOut
                    );
                }
                let runtime = dependencies.runtime.lock().await;
                let mut states = owner.spell_states.lock().await;
                assert!(
                    states
                        .get_mut(&runtime, actor, session)
                        .ok_or("dead actor")?
                        .pending_training_checkpoint()
                        .is_some()
                );
            }

            // The death flush's try, durable before the death is priced.
            let mut flushed_skills = trained.skills();
            flushed_skills[2].1 += 1;
            let flushed =
                DurableBuildState::new("druid", trained.magic(), flushed_skills).map_err(debug)?;
            let durable = format!(
                "SELECT {} FROM game_character_build_state \
                 WHERE character_id = encode($1,'hex')::uuid",
                columns("")
            );
            if matches!(case, Case::ReleaseRejected | Case::LiveRejected) {
                // The store refuses the stale fence before any write. The first attempt keeps the
                // actor dead with nothing committed, and re-fences the proven-uncommitted
                // checkpoint (same occurrence) to the session's current authority.
                let mut settled = None;
                if case == Case::ReleaseRejected {
                    assert!(
                        !owner
                            .settle_released_death(actor, session, &mut settled)
                            .await,
                        "a release must not settle a death whose flush was refused"
                    );
                    assert!(settled.is_none());
                } else {
                    assert!(
                        owner
                            .respawn_after_death(actor, session, death)
                            .await
                            .is_none(),
                        "a live death must not settle a death whose flush was refused"
                    );
                }
                assert_eq!(death_receipts(&harness).await?, 0);
                assert_eq!(build_row(&harness, &durable).await?, trained);
                assert!(owner.spell_states.lock().await.is_dead(actor));
                let current = current_fence(&owner, session).await?;
                {
                    let runtime = dependencies.runtime.lock().await;
                    let mut states = owner.spell_states.lock().await;
                    let state = states
                        .get_mut(&runtime, actor, session)
                        .ok_or("dead actor")?;
                    let retained = state
                        .pending_training_checkpoint()
                        .ok_or("the refused checkpoint stays retained")?;
                    assert_eq!(retained.fence, current);
                    assert_eq!(retained.prepared.request.occurrence, occurrence(93)?);
                    assert!(state.has_training_to_flush());
                }
                if case == Case::ReleaseRejected {
                    // The retried release flushes the try under the current fence, then settles
                    // the death priced from it.
                    assert!(
                        owner
                            .settle_released_death(actor, session, &mut settled)
                            .await,
                        "the retried release settles once the re-fenced flush is durable"
                    );
                    assert_eq!(settled, Some(death));
                    assert_eq!(death_receipts(&harness).await?, 1);
                    let before = format!(
                        "SELECT {} FROM game_character_death_receipts \
                         WHERE character_id = encode($1,'hex')::uuid",
                        columns("_before")
                    );
                    assert_eq!(build_row(&harness, &before).await?, flushed);
                    let lost = u64::try_from(
                        crate::domain::death::death_experience_loss(20, 0, false).map_err(debug)?,
                    )?;
                    assert_eq!(
                        build_row(&harness, &durable).await?,
                        expected_loss(table, &flushed, LEVEL_20_EXPERIENCE, lost)?
                    );
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    return harness.cleanup().await;
                }
            }

            // The composed respawn: flush, priced death, placement, rebase and refill.
            let (_, vitals) = owner
                .respawn_after_death(actor, session, death)
                .await
                .ok_or("the death settles and the actor respawns")?;
            assert_eq!(vitals.health, vitals.max_health);
            assert!(!owner.spell_states.lock().await.is_dead(actor));

            // The death receipt: binding version 2, its before build holds the flushed try, and
            // its after build is the SKILLS-0 §3.6 loss of the receipt's experience share.
            let (binding, experience_before, experience_lost): (i32, i64, i64) = sqlx::query_as(
                "SELECT get_byte(command_binding, 0), experience_before, experience_lost \
                 FROM game_character_death_receipts WHERE character_id = encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(binding, 2);
            assert_eq!(u64::try_from(experience_before)?, LEVEL_20_EXPERIENCE);
            let lost = u64::try_from(
                crate::domain::death::death_experience_loss(20, 0, false).map_err(debug)?,
            )?;
            assert_eq!(u64::try_from(experience_lost)?, lost);
            let receipt = |suffix: &str| {
                format!(
                    "SELECT {} FROM game_character_death_receipts \
                     WHERE character_id = encode($1,'hex')::uuid",
                    columns(suffix)
                )
            };
            assert_eq!(build_row(&harness, &receipt("_before")).await?, flushed);
            let lowered = build_row(&harness, &receipt("_after")).await?;
            assert_eq!(
                lowered,
                expected_loss(table, &flushed, LEVEL_20_EXPERIENCE, lost)?
            );
            assert!(lowered.magic() < flushed.magic());
            assert!(lowered.skills()[2] < flushed.skills()[2]);
            // The durable build is the lowered one, and the respawn obligation is consumed.
            assert_eq!(build_row(&harness, &durable).await?, lowered);
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_pending_respawns \
                 WHERE character_id = encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(pending, 0);

            // The live actor continues from the lowered build: one more try saves on it.
            {
                let runtime = dependencies.runtime.lock().await;
                let mut states = owner.spell_states.lock().await;
                let state = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("respawned actor")?;
                assert_eq!(
                    state.character_facts().magic_level,
                    u32::from(lowered.magic().0)
                );
                assert!(!state.has_training_to_flush());
                state
                    .train_skill(SkillFamily::Sword, 2, formula, |_| 1)
                    .map_err(debug)?;
            }
            assert_eq!(
                owner.save_spell_training(actor, session, true).await,
                TrainingSave::Saved
            );
            let mut next_skills = lowered.skills();
            next_skills[2].1 += 1;
            assert_eq!(
                build_row(&harness, &durable).await?,
                DurableBuildState::new("druid", lowered.magic(), next_skills).map_err(debug)?
            );

            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    }
}
