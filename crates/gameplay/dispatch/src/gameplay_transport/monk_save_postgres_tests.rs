#![allow(clippy::expect_used)]

// Keep sorted by module name; insert in order, never append.
#[path = "equipment_effects_owner_tests/monk_scenario.rs"]
pub(in crate::gameplay_transport) mod equipment_scenarios;
#[path = "first_vocation_vitals_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod first_vocation_vitals;
#[path = "../../../../../apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs"]
mod fixture;
use crate::gameplay_transport::composed_test_owner::fresh_fixture;
#[path = "item_look_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod item_look;
#[path = "kill_reward_postgres_e2e_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod kill_reward_e2e;
#[path = "logout_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod logout_e2e;
#[path = "map_bundle_domain_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod map_bundle_domain;
#[path = "loop_map_pickup_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod map_pickup;
#[path = "quest_admission_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod quest_admission;
#[path = "quest_chest_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod quest_chest_postgres_tests;
#[path = "quest_log_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod quest_log_postgres_tests;
#[path = "quest_recovery_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod quest_recovery;
#[path = "quest_runtime_fixture.rs"]
mod quest_runtime_fixture;
#[path = "rune_use_bridge_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod rune_use_bridge;
#[path = "tutorial_vitals_postgres_fixture.rs"]
mod tutorial_vitals_fixture;
#[path = "vocationless_vitals_postgres_tests.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod vocationless_vitals;

// Every test file in `postgres_tests/` is mounted here by `build.rs`, so a new mechanic adds its
// file there instead of another `#[path] mod` line in this shared list.
include!(concat!(env!("OUT_DIR"), "/postgres_test_support_mounts.rs"));
include!(concat!(env!("OUT_DIR"), "/postgres_tests_mounts.rs"));

// Keep the module paths used by the digest-pinned reward qualification tests.
use self::quest_callbacks_fixtures as quest_callbacks;

use super::{ComposedFreshAdmission, MonkSave};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, GraceExpiryResult, SessionContinuity,
};
use crate::gameplay_transport::fresh_evidence::FreshEvidenceSource;
use fixture::{Harness, debug, fence, id, run};
use std::cell::Cell;
use std::time::Duration;
use tokio::sync::Mutex;
use {
    crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId},
    oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn maintain_ready(root: &crate::durability::DurabilityRoot) -> TestResult {
    // Direct test owners have no node maintenance loop to replace a retired holder.
    if root.has_ready_demand() {
        root.maintain_ready_once().await.map_err(debug)?;
    }
    Ok(())
}

pub async fn maintain_authority_ready<
    A: crate::gameplay_transport::connection::FreshAdmissionAuthority,
>(
    authority: &A,
) {
    if let Some(root) = authority.test_durability_root() {
        maintain_ready(root).await.expect("direct serve readiness");
    }
}

thread_local! {
    static HOST_NOW: Cell<Option<i64>> = const { Cell::new(None) };
    static SAVES: Cell<usize> = const { Cell::new(0) };
    static NOT_EXPIRED: Cell<usize> = const { Cell::new(0) };
}
pub fn host_now() -> Option<i64> {
    HOST_NOW.get()
}
pub fn record_save_attempt() {
    SAVES.set(SAVES.get() + 1);
}
pub fn record_not_expired() {
    NOT_EXPIRED.set(NOT_EXPIRED.get() + 1);
}
fn reset() {
    HOST_NOW.set(None);
    SAVES.set(0);
    NOT_EXPIRED.set(0);
}

#[path = "monk_save_postgres_fixture/fresh_session.rs"]
#[cfg(oteryn_pg_shard_primary)]
mod fresh_session;
use fresh_session_fixtures::prepare_current_session;

struct Dependencies {
    runtime: Mutex<ChannelRuntimeV1>,
    clock_origin: std::time::Instant,
    room: crate::content::QualifiedNativeEntryRoom,
    combat_room: Option<crate::content::QualifiedNativeEntryRoom>,
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
        let world = fence(1)?.runtime_scope.world_id();
        Self::with_content(
            ChannelContentPin::test(world),
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )
    }

    fn with_content(
        pin: ChannelContentPin,
        room: crate::content::QualifiedNativeEntryRoom,
        spells: crate::spell::SpellBook,
    ) -> TestResult<Self> {
        Self::with_content_and_capacity(pin, room, spells, 2)
    }

    fn with_content_and_capacity(
        pin: ChannelContentPin,
        room: crate::content::QualifiedNativeEntryRoom,
        spells: crate::spell::SpellBook,
        actor_capacity: usize,
    ) -> TestResult<Self> {
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
            actor_capacity,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
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
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![CertificateDer::from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            PrivateKeyDer::try_from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        Ok(Self {
            runtime: Mutex::new(runtime),
            clock_origin: std::time::Instant::now(),
            room,
            combat_room: None,
            door: Mutex::new(door),
            evidence: FreshEvidenceSource::new(descriptor),
            spells,
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
        self.owner_from(&harness.root, &harness.node, authority)
    }
    fn owner_from<'a, 'f, 's>(
        &'a self,
        root: &'a crate::durability::DurabilityRoot,
        holder: &'a crate::durability::runtime_scope_assignment::NodeIncarnationProof,
        authority: &'a ReconciledCharacterAuthority<'f, 's>,
    ) -> ComposedFreshAdmission<'a, 'f, 's> {
        let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
            root,
            character: authority,
            holder,
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
            door: &self.door,
            spells: &self.spells,
            achievements: &self.achievements,
            imported_charms: &self.charms,
            room: &self.room,
        }
        .build();
        // Rebuilt session projections share the retained runtime's monotonic clock.
        owner.clock_origin = self.clock_origin;
        // Combat suites (creature move, kill reward) install their bundle combat view here.
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, self.combat_room.as_ref());
        owner
    }
    async fn initialize(&self, owner: &ComposedFreshAdmission<'_, '_, '_>) {
        self.initialize_player(owner, self.actor, self.admitted.game_session_id)
            .await;
    }
    async fn initialize_player(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let runtime = self.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
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

#[cfg(oteryn_pg_shard_primary)]
#[test]
fn boundary_release_retries_not_expired_and_saves_new_remaining_serene() -> TestResult {
    kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
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

/// Polls `release` until `reached`, then cancels it and waits for the root holder.
/// A release that ends first is returned as the error.
async fn drive_until<F: std::future::Future>(
    root: &crate::durability::DurabilityRoot,
    release: F,
    reached: impl Fn() -> bool,
) -> Result<(), F::Output> {
    use std::task::Poll;
    let mut release = Box::pin(release);
    let mut probe = std::pin::pin!(async {
        while !reached() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    let result = std::future::poll_fn(|context| {
        if let Poll::Ready(output) = release.as_mut().poll(context) {
            return Poll::Ready(Err(output));
        }
        if probe.as_mut().poll(context).is_ready() {
            return Poll::Ready(Ok(()));
        }
        Poll::Pending
    })
    .await;
    drop(release);
    // Cancellation can return the holder on SQLx's spawned task or retire it
    // during observed return. Use fixture maintenance to await an idle holder.
    root.request_ready();
    let ready = tokio::time::timeout(Duration::from_secs(10), root.maintain_ready_once()).await;
    assert!(
        matches!(ready, Ok(Ok(true))) && root.is_ready(),
        "cancelled monk release did not ready root {} within ten seconds: {ready:?}",
        root.root_identity(),
    );
    result
}

#[cfg(oteryn_pg_shard_primary)]
#[test]
fn boundary_unknown_monk_save_keeps_lease_until_store_recovers() -> TestResult {
    kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
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
            drive_until(
                &harness.root,
                owner.release_after_grace(dependencies.admitted),
                || SAVES.get() >= 2,
            ),
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

#[cfg(oteryn_pg_shard_primary)]
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

#[cfg(oteryn_pg_shard_primary)]
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

#[cfg(oteryn_pg_shard_primary)]
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

#[cfg(oteryn_pg_shard_primary)]
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

/// D58 / NIGHT-W1 decision 19 end to end on PostgreSQL through the
/// composed owner: a trained level 20 druid takes a real lethal creature hit, and
/// `respawn_after_death` force-saves its pending training, settles the death with the active
/// generation's training formula (all build values stay unchanged), places and consumes the
/// respawn, and keeps the live actor's trained
/// build, so its next training save commits on that build. The production generation supplies
/// the training formula and the progression; only its training content revision is the fixture
/// Character's interpretation (production pairs both as `s3b-content-1`).
mod death_ml_loss_composed {
    pub(super) use super::death_ml_loss_composed_fixtures::production_generation_with_pin;

    #[cfg(oteryn_pg_shard_primary)]
    use super::TestResult;
    #[cfg(oteryn_pg_shard_primary)]
    use super::death_ml_loss_composed_fixtures::{Case, SCENARIO_STACK_BYTES, scenario};

    #[cfg(oteryn_pg_shard_primary)]
    #[test]
    fn composed_owner_death_preserves_build_and_trains_on_postgres() -> TestResult {
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
    #[cfg(oteryn_pg_shard_primary)]
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
    #[cfg(oteryn_pg_shard_primary)]
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
    #[cfg(oteryn_pg_shard_primary)]
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
}
