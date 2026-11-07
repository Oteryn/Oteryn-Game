//! S04a: current consent and real fresh admission/terminal logout across two assigned nodes.
use super::chat_world_fixture as world;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::chat::relay::{RelayKey, RelayKeyRing};
use crate::durability::guild_chat_fixture as characters;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    chat_intent::ChatRuntime,
    chat_relay::ChatRelay,
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
    logout::LogoutReply,
    spell_premium_coordinator::{SpellPremiumCoordinator, purchase_fixture},
};
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::vip_presence::{self as wire, PresenceBatch, Status};
use std::{future::Future, sync::Arc, time::Duration};

#[path = "vip_presence/wire.rs"]
mod commands;
#[path = "vip_presence/progression.rs"]
mod progression;

#[test]
fn vip_presence_postgres_cross_node_login_logout_consent_and_privacy() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().name("vip-presence-world".into()).stack_size(64 << 20)
        .spawn(|| run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "vip_presence_world", true, false).await?;
            prepare_current_session(&h).await?;
            {
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut first = Dependencies::new()?;
                let mut fence = super::fixture::fence(1)?.item_fence();
                fence.character_lease_generation = 2;
                first.admitted.item_fence = Some(fence);
                {
                    let mut runtime = first.runtime.lock().await;
                    runtime.initialize_first_entry_position(first.actor).map_err(debug)?;
                    runtime.bind_attacker_lease(first.actor, first.admitted.game_session_id,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2,
                        ).map_err(debug)?).map_err(debug)?;
                }
                first.admitted.first_entry = FirstEntryOutcome::Positioned;
                let (consumer, clock, _, _) = purchase_fixture::current(id(40));
                let premium = SpellPremiumCoordinator::new(consumer, Some(clock), None, Some(h.root.clone()));
                let mut owner = first.owner(&h, &authority);
                owner.premium_coordinator = Some(&premium);
                first.initialize(&owner).await;
                let mut admitted = select(first.admitted)?;
                let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [37; 32])]).map_err(debug)?;
                let options = h.root.chat_relay_connect_options().ok_or("relay options")?;
                let relay = Arc::new(ChatRelay::new(ring.clone(), id(42), options.clone()).map_err(debug)?);
                owner.states.chat.install_relay(relay.clone());
                // Both nodes boot their real listeners before the target's fresh admission.
                // The target's inbound queue is independent of the observer; this case tests
                // its outgoing presence, using this same relay in the admitted target owner.
                let target_inbound = ChatRuntime::default();
                let target_relay = Arc::new(ChatRelay::new(ring, id(42), options).map_err(debug)?);
                let target_stop = CancellationToken::new();
                let target_listener = target_relay.run(&target_inbound, &target_stop);
                let stop = CancellationToken::new();
                let listener = relay.run(&owner.states.chat, &stop);
                let scenario = async {
                    while !relay.is_ready() || !target_relay.is_ready() {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    // Persisted, previously consented relationship before the target's next
                    // login. Only fixture DATA is historical; the login below uses the real
                    // fresh-admission writer on an independently assigned node and Channel.
                    characters::historical_character(&h.pool, 501,
                        crate::domain::CharacterId::from_bytes(id(world::RECIPIENT)).map_err(debug)?,
                        id(world::RECIPIENT_ACCOUNT), id(42), world::RECIPIENT_NAME).await?;
                    let add_payload = commands::named(1, world::RECIPIENT_NAME);
                    let add_command = admitted.continuity.next_command_id;
                    let added = commands::run(&owner, &mut admitted, 48, &add_payload).await?;
                    assert_eq!(commands::fields(&added)?["code"], 1);
                    let entry = commands::fields(&added)?["number"].as_u64().ok_or("entry number")?;
                    let edit = commands::edit(entry);
                    assert_eq!(commands::fields(&commands::run(&owner, &mut admitted, 48, &edit).await?)?["code"], 1);
                    sqlx::query("UPDATE game_vip_consents SET consent=2 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND inviter_character_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid")
                        .bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(world::RECIPIENT).as_slice()).execute(&h.pool).await?;
                    owner.states.chat.vip.forget(admitted.game_session_id);
                    let offline = observe(&owner, admitted, Status::Offline).await?;
                    assert_eq!(offline.entries[0].entry, entry);
                    assert!(!offline.entries[0].announce);

                    assert!(relay.is_ready());
                    assert!(target_relay.is_ready());
                    let (node, second) = world::admit_existing_second_node(&h).await?;
                    let mut target = second.owner(&h, &authority);
                    target.holder = &node;
                    assert_ne!(owner.holder.fact().node_id(), target.holder.fact().node_id());
                    assert_ne!(owner.channel_id, target.channel_id);
                    assert_eq!(owner.world_id, target.world_id);
                    progression::initialize(&target, second.admitted).await?;
                    second.initialize(&target).await;
                    let mut target_admitted = second.admitted;
                    target_admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 43],
                    ).ok_or("target without CHAT selection")?;
                    assert_eq!(target_admitted.continuity.selected_capabilities.as_slice(), &[6, 43]);
                    let target_character = oteryn_protocol_oteryn::CharacterId::decode(&id(world::RECIPIENT)).map_err(debug)?;
                    target.states.chat.install_relay(target_relay.clone());
                    let exercise = async {
                        assert!(!target.states.chat.knows(target_admitted.game_session_id));
                        // The session-row transition has committed; publication uses the same
                        // production lifecycle hook even though the target did not select CHAT.
                        target.vip_presence_publish_target(target_character, true).await;
                        let login = observe(&owner, admitted, Status::Online).await?;
                        assert!(login.entries[0].announce);
                        assert_eq!(login.entries[0].confirmed_for_ms, wire::CONFIRMED_FOR_MS);
                        let online_rows = h.root.read_current_vip_presence(&authority, &h.node, fence).await?;
                        assert_eq!(online_rows[0].status, Status::Online);

                        // Actual target command and committed row: hidden remains offline even
                        // under a matching/adversarial online hint, and cannot announce logout.
                        assert_eq!(commands::fields(&commands::run(&target, &mut target_admitted, 49, &commands::numbered(11, 2)).await?)?["code"], 1);
                        target.vip_presence_publish_target(target_character, true).await;
                        let hidden = observe(&owner, admitted, Status::Offline).await?;
                        assert!(!hidden.entries[0].announce);
                        assert_eq!(hidden.entries[0].confirmed_for_ms, 0);
                        assert_eq!(commands::fields(&commands::run(&target, &mut target_admitted, 49, &commands::numbered(11, 1)).await?)?["code"], 1);
                        target.vip_presence_publish_target(target_character, true).await;
                        assert!(!observe(&owner, admitted, Status::Online).await?.entries[0].announce);

                        // Stale generations and absent capability cannot read this player's
                        // contacts, even while a valid current observation is cached locally.
                        let mut stale = fence;
                        stale.character_lease_generation += 1;
                        assert!(h.root.read_current_vip_presence(&authority, &h.node, stale).await.is_err());
                        let mut unselected = admitted;
                        unselected.continuity.selected_capabilities = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[43]).ok_or("base VIP selection")?;
                        assert_eq!(unselected.continuity.selected_capabilities.as_slice(), &[43]);
                        assert!(owner.vip_presence_refresh(unselected).await.is_none());

                        // VIP/privacy commands must not create a target CHAT binding. Real
                        // logout still commits TERMINAL and publishes the canonical target.
                        assert!(!target.states.chat.knows(target_admitted.game_session_id));
                        assert_eq!(target.logout(target_admitted).await, LogoutReply::Result(LogoutResult::Accepted));
                        let ended: i16 = sqlx::query_scalar("SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
                            .bind(id(world::RECIPIENT_SESSION).as_slice()).fetch_one(&h.pool).await?;
                        assert_eq!(ended, 3);
                        let saved: (String, i64) = sqlx::query_as("SELECT r.character_revision::text,(SELECT count(*) FROM game_character_monk_state_receipts m WHERE m.character_id=r.character_id) FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid")
                            .bind(id(world::RECIPIENT).as_slice()).fetch_one(&h.pool).await?;
                        assert_eq!(saved, ("2".to_owned(), 1), "real terminal path must save the target's Monk state once");
                        let logout = observe(&owner, admitted, Status::Offline).await?;
                        assert!(logout.entries[0].announce);

                        // A committed withdrawal reuses the VIP command owner and removes the
                        // observing Character's consent. Its query shows no accepted status.
                        assert_eq!(commands::fields(&commands::run(&owner, &mut admitted, 49, &commands::numbered(10, entry)).await?)?["code"], 1);
                        let revoked = observe(&owner, admitted, Status::Unconfirmed).await?;
                        assert!(!revoked.entries[0].announce);
                        let consents: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_consents WHERE inviter_character_id=encode($1,'hex')::uuid")
                            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                        assert_eq!(consents, 0);

                        // A stopped actual listener blocks new mutations, but immutable old
                        // command outcomes still replay byte-for-byte without applying twice.
                        stop.cancel();
                        while relay.is_ready() {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        let replay = owner.vip_command(admitted, add_command, 48, &add_payload).await.ok_or("VIP durable replay while unavailable")?;
                        assert_eq!(replay, added);
                        let refused_command = admitted.continuity.next_command_id;
                        let refused = commands::run(&owner, &mut admitted, 48, &edit).await?;
                        assert_eq!(commands::fields(&refused)?["code"], 9);
                        let stored: serde_json::Value = sqlx::query_scalar("SELECT result FROM game_vip_command_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                            .bind(admitted.game_session_id.as_bytes().as_slice()).bind(refused_command.to_string()).fetch_one(&h.pool).await?;
                        assert_eq!(stored["code"], 9);
                        let retained_revision: i64 = sqlx::query_scalar("SELECT revision FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
                            .bind(id(40).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;

                        // A fresh runtime owner has no ingress cache. The stopped-listener
                        // refusal still replays, then remains final after a genuine LISTEN
                        // restart passes its stored lease/freshness floor.
                        let recovered = first.owner(&h, &authority);
                        recovered.states.chat.install_relay(relay.clone());
                        assert_eq!(recovered.vip_command(admitted, refused_command, 48, &edit).await.ok_or("unavailable receipt after runtime loss")?, refused);
                        let recovered_stop = CancellationToken::new();
                        let recovered_listener = relay.run(&recovered.states.chat, &recovered_stop);
                        let verify_recovery = async {
                            while !relay.is_ready() {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                            assert_eq!(recovered.vip_command(admitted, refused_command, 48, &edit).await.ok_or("unavailable receipt after listener recovery")?, refused);
                            // The original request hash includes its operation and target.
                            // Reusing this command for Remove must not delete the entry.
                            let changed = recovered.vip_command(admitted, refused_command, 48, &commands::numbered(2, entry)).await.ok_or("changed unavailable request")?;
                            assert_eq!(commands::fields(&changed)?["code"], 10);
                            let unchanged: (i64, i64) = sqlx::query_as("SELECT l.revision,(SELECT count(*) FROM game_vip_entries e WHERE e.account_id=l.account_id AND e.world_id=l.world_id AND e.entry_no=$3) FROM game_vip_lists l WHERE l.account_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid")
                                .bind(id(40).as_slice()).bind(id(42).as_slice()).bind(i64::try_from(entry)?).fetch_one(&h.pool).await?;
                            assert_eq!(unchanged, (retained_revision, 1));
                            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_command_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                                .bind(admitted.game_session_id.as_bytes().as_slice()).bind(refused_command.to_string()).fetch_one(&h.pool).await?;
                            assert_eq!(receipts, 1);
                            recovered_stop.cancel();
                            Ok::<(), Box<dyn std::error::Error>>(())
                        };
                        drive(recovered_listener, verify_recovery).await?;
                        target_stop.cancel();
                        Ok::<(), Box<dyn std::error::Error>>(())
                    };
                    exercise.await
                };
                tokio::time::timeout(
                    Duration::from_micros(2 * (crate::chat::relay::FRESHNESS_MICROS + crate::chat::relay::RELAY_LEASE_MICROS)) + Duration::from_secs(60),
                    drive(listener, drive(target_listener, scenario)),
                ).await??;
            }
            h.cleanup().await?;
            eprintln!("VIP_PRESENCE POSTGRES_CASE_COMPLETED: assigned nodes, fresh login, terminal logout, privacy, consent, fences, listener recovery and durable refusal replay");
            Ok(())
        }).map_err(|error| error.to_string()))?
        .join().map_err(|_| "VIP presence PG case panicked")?.map_err(Into::into)
}

fn select(mut admitted: AdmittedSession) -> TestResult<AdmittedSession> {
    // As in the existing VIP/Party PG fixtures, chat custody uses the published
    // bundle offer while map loading is outside this case's synthetic runtime.
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 7, 43, 258]).ok_or("VIP presence selection")?;
    assert_eq!(admitted.continuity.selected_capabilities.as_slice(), &[6, 7, 43, 258]);
    Ok(admitted)
}

async fn observe(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession, status: Status) -> TestResult<PresenceBatch> {
    loop {
        if let Some(bytes) = owner.vip_presence_refresh(admitted).await {
            let batch = wire::decode(&bytes).map_err(debug)?;
            if batch.entries.first().is_some_and(|entry| entry.status == status) {
                return Ok(batch);
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn drive<L: Future<Output = ()>, F: Future>(listener: L, scenario: F) -> F::Output {
    let mut listener = std::pin::pin!(listener);
    let mut scenario = std::pin::pin!(scenario);
    let mut listening = true;
    std::future::poll_fn(|cx| {
        // Listener-down/recovery assertions continue after the original run ends.
        // A completed async listener must never be polled a second time.
        if listening && listener.as_mut().poll(cx).is_ready() {
            listening = false;
        }
        scenario.as_mut().poll(cx)
    }).await
}
