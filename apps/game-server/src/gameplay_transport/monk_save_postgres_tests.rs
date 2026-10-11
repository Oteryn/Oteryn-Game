#![allow(clippy::expect_used)]

#[path = "../../tests/support/character_revision_sequencer_postgres_cases.rs"]
mod fixture;

#[path = "monk_save_fresh_fixture.rs"]
mod fresh_fixture;

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
            runtime: &self.runtime,
            movement_cells: self.room.movement_cells(),
            door: &self.door,
            chest: self.room.door(),
            spells: &self.spells,
            active_generation: None,
            premium_coordinator: None,
            qualified_room: None,
            achievements: &self.achievements,
            imported_charms: &self.charms,
            spell_states: Mutex::default(),
            attack: Mutex::default(),
            clock_origin: std::time::Instant::now(),
            lost: std::sync::Mutex::default(),
            revision_sequencer:
                crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
            quest_catalogue: None,
            quest_sessions: std::sync::Mutex::default(),
            chat: crate::gameplay_transport::chat_intent::ChatRuntime::default(),
            wheel_ruleset: None,
            wheel_sessions: std::sync::Mutex::default(),
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
        assert!(
            tokio::time::timeout(
                Duration::from_millis(600),
                owner.release_after_grace(dependencies.admitted)
            )
            .await
            .is_err()
        );
        assert!(
            SAVES.get() >= 1,
            "the real release loop must reach the monk save boundary"
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
