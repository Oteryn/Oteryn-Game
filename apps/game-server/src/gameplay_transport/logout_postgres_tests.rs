//! LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6, §2.2) on PostgreSQL: the real composed
//! owner's logout of the fresh-admission fixture session. The terminal release commits through
//! the exact fenced store path, writes the actor's final position (CHAR-POSITION-1) and retires
//! the actor; a refusal changes nothing; an unknown outcome is reconciled from the row.
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.

use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset, revision, track};
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionState};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    ControlLossResult, FreshAdmissionAuthority as _, GraceExpiryResult, LogoutReply,
};
use oteryn_protocol_oteryn::logout::LogoutResult;

async fn session_state(
    harness: &Harness,
    dependencies: &Dependencies,
) -> TestResult<GameSessionState> {
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(dependencies.admitted.game_session_id)
        .await
        .map_err(debug)?;
    Ok(current.session_state())
}

async fn last_positions(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions")
            .fetch_one(&harness.pool)
            .await?,
    )
}

async fn actor_present(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> bool {
    owner
        .runtime
        .lock()
        .await
        .player_control_facts(dependencies.actor, dependencies.admitted.game_session_id)
        .is_ok()
}

/// The admitted actor stands on its first-entry cell and its session is tracked, so the
/// release has a final position to write.
async fn enter(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> TestResult {
    dependencies.initialize(owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(dependencies.actor)
        .map_err(debug)?;
    track(owner, dependencies)
}

#[test]
fn logout_commits_terminal_saves_the_position_and_retires_the_actor() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_accepted", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        assert_eq!(last_positions(&harness).await?, 0);

        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        // §1.6: TERMINAL with no grace and no loss epoch; the actor has left the Channel.
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Terminal
        );
        let terminal: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_durability_reconnect_sessions \
             WHERE session_state=3 AND control_loss_epoch IS NULL",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(terminal, 1, "TERMINAL with no loss epoch");
        assert!(!actor_present(&owner, &dependencies).await);
        // CHAR-POSITION-1: the release wrote the fenced actor's cell.
        assert_eq!(last_positions(&harness).await?, 1);
        // The monk save point ran before the lease ended.
        assert_eq!(revision(&harness).await?, 2);

        // A replay of the release is TERMINAL, never a second commit.
        let replay = FreshAdmissionStore::from_root(harness.root.clone())
            .release_logout_session(
                dependencies.admitted.game_session_id,
                &crate::gameplay_transport::canonical_uuid(&id(40)),
                dependencies
                    .admitted
                    .controller
                    .ok_or("controller")?
                    .transport,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(replay, ExpiredLossReleaseV1::Terminal));
        // A second logout of the ended session refuses at once: there is no actor left.
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Busy)
        );
        assert_eq!(revision(&harness).await?, 2);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// VIP-2 (VIP0 §8.2): the owner's relay, unreachable so that hints stay in its outbox for the
/// test to read. The character is the fixture's (`id(41)`) and its session has published online
/// through the helper the fresh admission calls after its commits.
fn presence_relay<'o>(
    owner: &'o ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> TestResult<(
    &'o crate::gameplay_transport::chat_relay::ChatRelay,
    oteryn_protocol_oteryn::CharacterId,
)> {
    use std::str::FromStr as _;
    // The relay's World only names its channel here; nothing is sent.
    let world = [
        0x01, 0x90, 0x0a, 0x5d, 0xac, 0x96, 0x7a, 0xb2, 0x80, 0xe7, 0x01, 0x23, 0x45, 0x67, 0x89,
        0xab,
    ];
    let ring = crate::gameplay_transport::chat_relay::parse_keys(
        format!(
            "1 0 {}\n",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [1_u8; 32])
        )
        .as_bytes(),
    )
    .map_err(debug)?;
    let options =
        sqlx::postgres::PgConnectOptions::from_str("postgresql://nobody@127.0.0.1:1/none")?;
    let relay = crate::gameplay_transport::chat_relay::ChatRelay::new(ring, world, options)
        .map_err(debug)?;
    owner.chat.install_relay(std::sync::Arc::new(relay));
    let relay = owner.chat.relay().ok_or("installed relay")?;
    let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
    owner.presence_login(dependencies.admitted.game_session_id, character);
    let character = oteryn_protocol_oteryn::CharacterId::decode(&id(41)).map_err(debug)?;
    assert_eq!(relay.queued_presence(), vec![(character, true)]);
    Ok((relay, character))
}

/// VIP-2 (VIP0 §8.2): B is admitted (online queued), then B's real logout commits TERMINAL and
/// retires the actor, which queues B's offline hint for the watchers' nodes.
#[test]
fn an_accepted_logout_sends_the_offline_presence_hint() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_presence", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        let (relay, character) = presence_relay(&owner, &dependencies)?;
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Terminal
        );
        // The online hint not yet sent is replaced in place by the newest transition.
        assert_eq!(relay.queued_presence(), vec![(character, false)]);
        // The refused second logout retires nothing and queues nothing more.
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Busy)
        );
        assert_eq!(relay.queued_presence(), vec![(character, false)]);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// VIP-2 (VIP0 §8.2): a recorded control loss releases the session's presence watcher, so a
/// session that waits in grace holds none of the owner's capacity; a resume's replacement
/// snapshot starts a new one.
#[test]
fn a_recorded_control_loss_releases_the_vip_presence_watcher() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_vip_watcher", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        let presence = std::sync::Arc::new(
            crate::durability::vip_presence::PresenceOwner::new(
                harness.root.clone(),
                owner.world_id,
                1,
            )
            .map_err(debug)?,
        );
        presence.listener_reconnected().map_err(debug)?;
        assert!(
            owner
                .chat
                .vip
                .install_presence(std::sync::Arc::clone(&presence))
        );
        let session = dependencies.admitted.game_session_id;
        let gameplay = fence(1)?;
        presence
            .reserve_for_test(
                crate::durability::item_transfer::CurrentCharacterItemFence {
                    character_id: gameplay.character_id,
                    game_session_id: session,
                    connection_generation: gameplay.connection_generation,
                    character_lease_generation: gameplay.character_lease_generation,
                    runtime_scope: gameplay.runtime_scope,
                    scope_ownership_generation: gameplay.scope_ownership_generation,
                },
            )
            .map_err(debug)?;
        assert!(presence.watches(session).map_err(debug)?);
        assert_eq!(
            owner
                .lose_control(dependencies.admitted, std::time::Duration::ZERO)
                .await,
            ControlLossResult::Recorded
        );
        assert!(!presence.watches(session).map_err(debug)?);
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Reconnectable
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn logout_in_fight_is_refused_and_changes_nothing() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_in_fight", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        let (relay, character) = presence_relay(&owner, &dependencies)?;
        let now = owner.owner_now();
        {
            let runtime = owner.runtime.lock().await;
            owner.attack.lock().await.record_hit_taken(
                &runtime,
                dependencies.actor,
                dependencies.admitted.game_session_id,
                now,
            );
        }
        let until = owner
            .attack
            .lock()
            .await
            .in_fight_until(
                dependencies.actor,
                dependencies.admitted.game_session_id,
                now,
            )
            .ok_or("the hit starts the in-fight deadline")?;

        let LogoutReply::Result(LogoutResult::InFight { retry_after_ms }) =
            owner.logout_session(dependencies.admitted).await
        else {
            return Err("an in-fight logout is IN_FIGHT".into());
        };
        // The retry is the deadline left, in whole milliseconds rounded up, read after `now`.
        let left_ms = until.get().saturating_sub(now.get()).div_ceil(1_000);
        assert!(retry_after_ms >= 1 && u64::from(retry_after_ms) <= left_ms);
        assert!(left_ms - u64::from(retry_after_ms) <= 1_000);
        // Nothing changed: the session is ACTIVE, the actor present, nothing written.
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Active
        );
        assert!(actor_present(&owner, &dependencies).await);
        assert_eq!(last_positions(&harness).await?, 0);
        assert_eq!(revision(&harness).await?, 1);
        // VIP-2: the character is still online; no offline hint.
        assert_eq!(relay.queued_presence(), vec![(character, true)]);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn logout_of_a_superseded_controller_is_busy_and_lifts_its_fence() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_stale", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        // A newer controller generation holds the session on another transport: the logout's
        // stale generation releases nothing.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET current_transport_ref=$1, \
             current_generation=current_generation+1 WHERE game_session_id=encode($2,'hex')::uuid",
        )
        .bind([0x0b_u8; 16].as_slice())
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Busy)
        );
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Active
        );
        assert!(actor_present(&owner, &dependencies).await);
        // The store itself refuses the stale transport.
        assert!(matches!(
            FreshAdmissionStore::from_root(harness.root.clone())
                .release_logout_session(
                    dependencies.admitted.game_session_id,
                    &crate::gameplay_transport::canonical_uuid(&id(40)),
                    dependencies
                        .admitted
                        .controller
                        .ok_or("controller")?
                        .transport,
                )
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::NotApplicable
        ));

        // The BUSY lifted the fence: once the session is this controller's again, a retry
        // fences and commits.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET current_transport_ref=$1, \
             current_generation=current_generation+1 WHERE game_session_id=encode($2,'hex')::uuid",
        )
        .bind([9_u8; 16].as_slice())
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Terminal
        );
        assert!(!actor_present(&owner, &dependencies).await);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn an_uncommitted_unknown_logout_becomes_an_ordinary_loss() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_unknown", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        // The release transaction cannot read the claim rows: its outcome is unknown.
        sqlx::query("ALTER TABLE game_durability_admission_guard_history RENAME TO logout_fixture_unavailable_guards").execute(&harness.pool).await?;
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Unknown
        );
        sqlx::query("ALTER TABLE logout_fixture_unavailable_guards RENAME TO game_durability_admission_guard_history").execute(&harness.pool).await?;
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Active
        );
        assert!(actor_present(&owner, &dependencies).await);

        // The connection closed with no result: the lifecycle lifts the kept fence and records
        // the transport loss as for any closed client.
        let controller = dependencies.admitted.controller.ok_or("controller")?;
        assert_eq!(
            owner
                .commit_control_loss(
                    dependencies.admitted.game_session_id,
                    dependencies.actor,
                    controller,
                )
                .await,
            ControlLossResult::Recorded
        );
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Reconnectable
        );
        assert!(actor_present(&owner, &dependencies).await);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_committed_logout_whose_acknowledgement_was_lost_is_reconciled_from_the_row() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_reconcile", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        let controller = dependencies.admitted.controller.ok_or("controller")?;
        // The release committed, but the owner never learned it: the actor is still present.
        assert!(matches!(
            FreshAdmissionStore::from_root(harness.root.clone())
                .release_logout_session(
                    dependencies.admitted.game_session_id,
                    &crate::gameplay_transport::canonical_uuid(&id(40)),
                    controller.transport,
                )
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::Released { .. }
        ));
        assert!(actor_present(&owner, &dependencies).await);

        // The lifecycle finds the row TERMINAL and retires the actor; nothing is resumed.
        assert_eq!(
            owner
                .commit_control_loss(
                    dependencies.admitted.game_session_id,
                    dependencies.actor,
                    controller,
                )
                .await,
            ControlLossResult::Terminal
        );
        assert_eq!(
            owner
                .reconcile_terminal_session(dependencies.admitted)
                .await,
            GraceExpiryResult::Released
        );
        assert!(!actor_present(&owner, &dependencies).await);
        // A repeat (another reconcile pass) finds the slot gone and still settles.
        assert_eq!(
            owner
                .reconcile_terminal_session(dependencies.admitted)
                .await,
            GraceExpiryResult::Released
        );
        // The ended session's stale generation cannot write its character any more.
        assert_eq!(
            owner
                .save_monk_state(&dependencies.admitted, dependencies.actor)
                .await,
            super::MonkSave::FencedOut
        );
        assert_eq!(revision(&harness).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_transport_other_than_the_current_one_never_releases() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_transport", true, false).await?;
        prepare_current_session(&harness).await?;
        let store = FreshAdmissionStore::from_root(harness.root.clone());
        let session = crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?;
        let account = crate::gameplay_transport::canonical_uuid(&id(40));
        let other = AuthenticatedTransportRefV1::decode(&[0x0b; 16]).map_err(debug)?;
        assert!(matches!(
            store
                .release_logout_session(session, &account, other)
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::NotApplicable
        ));
        // A RECONNECTABLE session is not a logout's either.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1, current_transport_ref=NULL, control_loss_epoch=1, predecessor_generation=current_generation, original_grace_deadline=extract(epoch FROM clock_timestamp())::bigint + 60 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(50).as_slice()).execute(&harness.pool).await?;
        let own = AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?;
        assert!(matches!(
            store
                .release_logout_session(session, &account, own)
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::NotApplicable
        ));
        let (current, _) = store.current_session_at(session).await.map_err(debug)?;
        assert_eq!(current.session_state(), GameSessionState::Reconnectable);
        harness.cleanup().await
    })
}

/// The debug-build `serve_admitted` future is deeper than the default 2 MiB test stack, so the
/// VIP dispatch scenario runs on a dedicated thread with an explicit stack (as the kill reward
/// e2e).
const VIP_DISPATCH_STACK_BYTES: usize = 64 << 20;

/// VIP-2 (VIP0 §8.2) through the real composed owner and `serve_admitted` on PostgreSQL: an
/// accept commits and its acknowledgement is lost; the presence listener goes down and the
/// replacement snapshot no longer names the accepted handle. The resend of the receipted command
/// replays from its receipt, a new command is refused, nothing is written, and a receipt lookup
/// that cannot finish leaves the outcome unknown.
#[test]
fn vip_listener_down_replays_a_committed_accept_through_dispatch() -> TestResult {
    std::thread::Builder::new()
        .name("vip-dispatch-replay".into())
        .stack_size(VIP_DISPATCH_STACK_BYTES)
        .spawn(|| vip_listener_down_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "VIP dispatch replay thread panicked")?
        .map_err(Into::into)
}

async fn vip_footprint(harness: &Harness) -> TestResult<(i64, i64, i64, u64)> {
    let (entries, consents, receipts): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_vip_entries),(SELECT count(*) FROM game_vip_consents),\
         (SELECT count(*) FROM game_vip_command_receipts)",
    )
    .fetch_one(&harness.pool)
    .await?;
    Ok((entries, consents, receipts, revision(harness).await?))
}

fn vip_listener_down_scenario() -> TestResult {
    use crate::durability::item_transfer::CurrentCharacterItemFence;
    use crate::foundation::{CommandStatus, MessageType};
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use crate::gameplay_transport::connection::{
        AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, SessionContinuity,
        UseCommand, serve_admitted,
    };
    use crate::gameplay_transport::vip_intent::durability_vip_tests::{
        insert_vip_character, insert_vip_invitation,
    };
    use oteryn_protocol_oteryn::vip::{
        CAPABILITY_VIP_V1, COMMAND_TYPE_VIP_INTENT, VipIntent, VipResultCode,
        decode_vip_intent_result, encode_vip_intent,
    };
    use oteryn_protocol_oteryn::{
        ClientCommandValue, decode_command_result, decode_wire_envelope, encode_client_command,
    };
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_vip_replay", true, false).await?;
        prepare_current_session(&harness).await?;
        // The session's character 41 (account 40) in World 42 holds a pending invitation from
        // character 60 of account 70.
        insert_vip_character(&harness.pool, id(41), [id(60), id(70), id(42)], 60).await?;
        insert_vip_invitation(&harness.pool, [id(70), id(42), id(60), id(41)], 1).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        let presence = std::sync::Arc::new(
            crate::durability::vip_presence::PresenceOwner::new(
                harness.root.clone(),
                owner.world_id,
                1,
            )
            .map_err(debug)?,
        );
        presence.listener_reconnected().map_err(debug)?;
        assert!(
            owner
                .chat
                .vip
                .install_presence(std::sync::Arc::clone(&presence))
        );
        let session = dependencies.admitted.game_session_id;
        let gameplay = fence(1)?;
        // VIP's store checks the item fence against the durable session, whose fresh admission
        // holds the character lease it committed.
        let lease: String = sqlx::query_scalar(
            "SELECT character_lease_generation::text FROM game_durability_reconnect_sessions \
             WHERE game_session_id=encode($1,'hex')::uuid",
        )
        .bind(session.as_bytes().as_slice())
        .fetch_one(&harness.pool)
        .await?;
        let lease = lease.parse()?;
        let item = CurrentCharacterItemFence {
            character_id: gameplay.character_id,
            game_session_id: session,
            connection_generation: gameplay.connection_generation,
            character_lease_generation: lease,
            runtime_scope: gameplay.runtime_scope,
            scope_ownership_generation: gameplay.scope_ownership_generation,
        };
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(item),
        };
        let snapshot = owner
            .vip_open(session, Some(item))
            .await
            .ok_or("VIP snapshot")?;
        let handle = snapshot
            .consent
            .invitations
            .first()
            .ok_or("invitation handle")?
            .handle;
        let payload =
            encode_vip_intent(&VipIntent::Accept { invitation: handle }).map_err(debug)?;
        // The accept commits; its acknowledgement is lost with the connection.
        let (committed, _) = owner.vip_command(command(1), &payload).await;
        assert!(!committed.unknown);
        assert_eq!(committed.result.code, VipResultCode::Ok);
        // The listener goes down; the replacement snapshot has lost the accepted handle.
        presence.listener_down().map_err(debug)?;
        assert!(!presence.listening());
        let snapshot = owner
            .vip_open(session, Some(item))
            .await
            .ok_or("VIP snapshot")?;
        assert!(snapshot.consent.invitations.is_empty());
        assert!(
            snapshot
                .consent
                .watchers
                .first()
                .ok_or("watcher handle")?
                .handle
                > handle
        );
        let before = vip_footprint(&harness).await?;
        let admitted = AdmittedSession {
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity {
                selected_capabilities: SelectedCapabilities::select(
                    &[OfferedCapability {
                        id: CAPABILITY_VIP_V1,
                        requires: &[],
                    }],
                    &[CAPABILITY_VIP_V1],
                )
                .ok_or("VIP capability selection")?,
                ..SessionContinuity::FRESH
            },
            item_fence: Some(item),
            ..dependencies.admitted
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        // The receipted command replays even without its old handle; a new command is refused.
        for command_id in 1..=2 {
            let frame = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_VIP_INTENT,
                    payload: &payload,
                },
            )?;
            client
                .write_all(&(frame.len() as u32).to_be_bytes())
                .await?;
            client.write_all(&frame).await?;
        }
        client.shutdown().await?;
        let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut results = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            let envelope = decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload())?;
                results.push((
                    result.command_id,
                    result.status,
                    decode_vip_intent_result(result.payload)
                        .map_err(debug)?
                        .code,
                ));
            }
            cursor += 4 + length;
        }
        assert_eq!(
            results,
            vec![
                (1, CommandStatus::Accepted, VipResultCode::Ok),
                (2, CommandStatus::Rejected, VipResultCode::VipUnavailable),
            ]
        );
        assert_eq!(vip_footprint(&harness).await?, before);
        // A receipt lookup that cannot finish leaves the outcome unknown: the CommandId stays
        // unsequenced, so a later resend can still replay the committed accept.
        let mut lock = harness.pool.begin().await?;
        sqlx::query("LOCK TABLE game_vip_command_receipts IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *lock)
            .await?;
        let (outcome, deltas) = owner.vip_command(command(1), &payload).await;
        lock.rollback().await?;
        assert!(outcome.unknown);
        assert_eq!(outcome.result.code, VipResultCode::VipUnavailable);
        assert!(deltas.is_empty());
        assert_eq!(vip_footprint(&harness).await?, before);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
