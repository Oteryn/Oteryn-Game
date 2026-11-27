//! A fresh GameSession can write Quick Loot destinations while the holder is contended.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CommandStatus, WorldId};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, UseCommand,
};
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
    purchase_fixture,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt as _;
use oteryn_protocol_oteryn::{logout::LogoutResult, quick_loot as wire};

#[allow(
    dead_code,
    reason = "the shared mTLS fixture serves other Premium cases"
)]
#[path = "../../../../social/src/premium/test_producer.rs"]
mod premium_test_producer;

#[test]
fn quick_loot_food_to_gold_pouch_waits_for_relog_snapshot_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::PlatformSnapshot)
}

#[test]
fn quick_loot_food_to_gold_pouch_on_first_command_after_relog() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::HolderContention)
}

#[test]
fn quick_loot_pouch_relog_releases_on_one_logout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::PouchLogout)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    HolderContention,
    PlatformSnapshot,
    PouchLogout,
}

fn run(admin: String, case: Case) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-relog", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(admin, case))
    })
}

async fn scenario(admin: String, case: Case) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "quick_loot_relog", false, false).await?;
    let result = exercise(&h, case).await;
    let cleanup = h.cleanup().await;
    result?;
    cleanup
}

async fn exercise(h: &Harness, case: Case) -> TestResult {
    let live_premium = case == Case::PlatformSnapshot;
    let request = prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active Content")?;
    let native = active.native_gameplay().ok_or("native Content")?;
    super::tutorial_vitals_fixture::publish_native_activation(h, &pin).await?;
    super::tutorial_vitals_fixture::native_asset_revisions(h, active).await?;
    let room = pin.qualified_room().clone();
    let deps =
        Dependencies::with_content(pin.into_channel_pin(), room, native.spell_book().clone())?;
    let (consumer, clock, activation) = if live_premium {
        let clock: std::sync::Arc<dyn RegisteredPremiumClock> = std::sync::Arc::new(SnapshotClock);
        (
            Default::default(),
            clock,
            PremiumActivation {
                activation_id: "quick-loot-relog-snapshot".into(),
                switch_over_us: SNAPSHOT_ISSUED_US,
            },
        )
    } else {
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        (consumer, clock, activation)
    };
    let premium = SpellPremiumCoordinator::new(
        consumer.clone(),
        Some(clock),
        Some(activation.clone()),
        Some(h.root.clone()),
    );
    let live = if live_premium {
        Some(LivePremium::start().await?)
    } else {
        None
    };
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    admitted.item_fence = crate::gameplay_transport::item_fence_of(
        store.current_session(admitted.game_session_id).await?,
    );
    owner.admit_progression_session(&mut admitted).await;
    super::track(&owner, &deps)?;
    let pouch = id(71);
    let pouch_key = "oteryn:item.tibia.i23721";
    let revision = native
        .item_index()
        .and_then(|index| index.revision(pouch_key))
        .ok_or("Gold Pouch definition")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($1,'hex')::uuid)")
            .bind(pouch.as_slice()).bind(id(42).as_slice()).bind(pouch_key).bind(revision)
            .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_character_inbox_locations(character_id,item_instance_id,world_id,ordinal)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1)")
            .bind(id(41).as_slice()).bind(pouch.as_slice()).bind(id(42).as_slice())
            .execute(&mut *tx).await?;
    tx.commit().await?;
    if case == Case::PouchLogout {
        let request = wire::QuickLootRequest {
            action: wire::QuickLootAction::AssignLoot,
            corpse: None,
            selected_item: None,
            review_filters: false,
            category: None,
            recipient: Some(wire::QuickLootRecipient::GoldPouch),
            recipient_id: Some(pouch),
            fallback_to_main: true,
            filter_mode: None,
            filter_definition_ref: None,
        };
        let payload = wire::encode_quick_loot_request(&request).map_err(debug)?;
        assert_eq!(
            owner
                .quick_loot_destinations(
                    admitted,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: admitted.continuity.next_command_id,
                        item_fence: admitted.item_fence,
                    },
                    (request.action.command_type(), &payload),
                    Some(request),
                )
                .await,
            Some((CommandStatus::Accepted, Vec::new()))
        );
    }
    if let Some(live) = &live {
        owner.premium = crate::premium::refresh::PremiumRefresher::new(
            consumer.clone(),
            h.root.clone(),
            Some(live.client.clone()),
        );
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), admitted.game_session_id);
        let proof = async {
            while !owner
                .session_premium_gate(admitted.game_session_id)
                .await
                .premium_current()
            {
                tokio::task::yield_now().await;
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(10), proof).await?;
        let record = h.root.load_premium_fence(id(40)).await?;
        assert_eq!(
            record.fence.ok_or("durable Premium proof")?.latest.state,
            crate::durability::premium_fence::EntitlementState::Active,
        );
        live.pause_next
            .store(true, std::sync::atomic::Ordering::Release);
    }
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );

    assert_eq!(
        store
            .current_session(admitted.game_session_id)
            .await?
            .session_state(),
        crate::foundation::GameSessionState::Terminal
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(deps.actor, admitted.game_session_id)
            .is_err()
    );
    let character = admitted
        .item_fence
        .ok_or("initial item fence")?
        .character_id;
    let monk = h
        .root
        .read_character_monk_state(&authority, character)
        .await
        .map_err(debug)?;
    assert_eq!(
        monk.harmony(),
        0,
        "a vocationless fixture must not save Monk Harmony"
    );
    assert_eq!(monk.serene_forced_remaining_micros(), 0);

    if live_premium {
        assert_eq!(
            consumer.refresh_after_us(id(40)),
            None,
            "logout drops in-memory proof"
        );
    }

    let (session, relog_request) = super::full_world_support::relogin_request(h).await?;
    let current = store.current_session(session).await?;
    let transport = current.current_transport().ok_or("relog transport")?;
    let actor = {
        let mut runtime = deps.runtime.lock().await;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        runtime.commit_fresh_session(reserved).map_err(debug)?
    };
    let (outcome, fence, _) =
        Box::pin(owner.initialize_first_entry(&relog_request, session, transport, actor)).await;
    assert!(
        matches!(
            outcome,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "{outcome:?}"
    );
    assert!(fence.is_some());
    if !live_premium {
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await?;
    }
    owner.premium_coordinator = Some(&premium);
    owner.admit_premium(id(40), session);
    admitted.game_session_id = session;
    admitted.runtime_actor = Some(actor);
    admitted.first_entry = outcome;
    admitted.item_fence = fence;
    admitted.controller = Some(ControllerBinding {
        transport,
        account_id: id(40),
    });
    admitted.continuity.house_input_pending = Some(false);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 98, 607])
            .ok_or("Quick Loot destination capability")?;

    if case == Case::PouchLogout {
        return logout_once(&owner, h, admitted).await;
    }

    let command = wire::QuickLootRequest {
        action: wire::QuickLootAction::AssignLoot,
        corpse: None,
        selected_item: None,
        review_filters: false,
        category: Some(wire::QuickLootCategory::Food),
        recipient: Some(wire::QuickLootRecipient::GoldPouch),
        recipient_id: Some(pouch),
        fallback_to_main: true,
        filter_mode: None,
        filter_definition_ref: None,
    };
    let payload = wire::encode_quick_loot_request(&command).map_err(debug)?;
    let command_type = command.action.command_type();
    if let Some(live) = live {
        return live
            .assign(&owner, &mut admitted, command_type, payload)
            .await;
    }
    let reply = {
        let holder = h.root.try_issue_semantic_pass()?;
        let mut routed = std::pin::pin!(owner.quick_loot_destinations(
            admitted,
            UseCommand {
                game_session_id: session,
                command_id: admitted.continuity.next_command_id,
                item_fence: fence,
            },
            (command_type, &payload),
            Some(command),
        ));
        // Let the command encounter the held slot before releasing it. A synchronous
        // refusal or disconnect here is the original failure.
        let early = std::future::poll_fn(|cx| {
            use std::future::Future as _;
            std::task::Poll::Ready(match routed.as_mut().poll(cx) {
                std::task::Poll::Ready(result) => Some(result),
                std::task::Poll::Pending => None,
            })
        })
        .await;
        drop(holder);
        match early {
            Some(reply) => reply,
            None => routed.await,
        }
    };
    eprintln!(
        "QUICK_LOOT_RELOG first assignment accepted={}",
        matches!(reply, Some((CommandStatus::Accepted, _)))
    );
    assert!(
        matches!(
            reply,
            Some((CommandStatus::Accepted, ref payload))
                if payload.is_empty()
        ),
        "first Food assignment after relog: {reply:?}"
    );
    let stale = wire::encode_quick_loot_request(&wire::QuickLootRequest {
        recipient_id: Some(id(72)),
        ..command
    })
    .map_err(debug)?;
    let review = wire::encode_quick_loot_request(&wire::QuickLootRequest {
        action: wire::QuickLootAction::ReviewDestinations,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        ..command
    })
    .map_err(debug)?;
    let (replies, _) = super::parameter_spell_fixtures::exchange(
        &owner,
        &mut admitted,
        &[
            (command_type, payload),
            (command_type, stale),
            (command_type, review),
        ],
        false,
        false,
    )
    .await?;
    assert_eq!(replies.len(), 3);
    assert_eq!(
        replies[0],
        (CommandStatus::Accepted, Vec::new()),
        "durable replay"
    );
    assert_eq!(replies[1].0, CommandStatus::Rejected);
    assert_eq!(
        wire::decode_quick_loot_refusal(&replies[1].1),
        Ok(wire::QuickLootRefusal::Stale)
    );
    assert_eq!(
        replies[2].0,
        CommandStatus::Accepted,
        "the refused command keeps the stream open"
    );
    let destinations = wire::decode_quick_loot_destinations(&replies[2].1).map_err(debug)?;
    assert!(destinations.loot.contains(&wire::QuickLootDestination {
        category: command.category,
        recipient: wire::QuickLootRecipient::GoldPouch,
        recipient_id: Some(pouch),
    }));
    Ok(())
}

// The shared producer's default snapshot starts at 2026-09-30T12:00:00Z.
const SNAPSHOT_ISSUED_US: i64 = 1_790_769_600_000_000;

struct SnapshotClock;
impl clock_registration::Registered for SnapshotClock {}
impl RegisteredPremiumClock for SnapshotClock {
    fn read_current_window(&self) -> Option<crate::premium::TrustedNow> {
        crate::premium::TrustedNow::new(SNAPSHOT_ISSUED_US + 60_000_000, 0)
    }
}

struct LivePremium {
    // The producer task is stopped by its owner on drop.
    _producer: premium_test_producer::TestProducer,
    client: crate::premium::client::PremiumSnapshotClient,
    pause_next: std::sync::Arc<std::sync::atomic::AtomicBool>,
    requested: std::sync::Arc<tokio::sync::Notify>,
    respond: std::sync::Arc<tokio::sync::Notify>,
}

impl LivePremium {
    async fn start() -> TestResult<Self> {
        use crate::premium::client::{PremiumClientConfig, PremiumSnapshotClient};
        use premium_test_producer::{Reply, TestPki, TestProducer, snapshot};
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let pause_next = Arc::new(AtomicBool::new(false));
        let requested = Arc::new(tokio::sync::Notify::new());
        let respond = Arc::new(tokio::sync::Notify::new());
        let pki = TestPki::new();
        let producer = TestProducer::start(&pki, {
            let (pause_next, requested, respond) =
                (pause_next.clone(), requested.clone(), respond.clone());
            move |request| {
                let reply = Reply::json(snapshot(request, 7, &[]));
                if pause_next.load(Ordering::Acquire) {
                    requested.notify_one();
                    reply.after_notification(respond.clone())
                } else {
                    reply
                }
            }
        })
        .await;
        let client = PremiumSnapshotClient::new(&PremiumClientConfig {
            origin: producer.origin.clone(),
            identity_pem: pki.client_identity_pem.into_bytes(),
            platform_ca_pem: pki.ca_pem.into_bytes(),
        })?;
        Ok(Self {
            _producer: producer,
            client,
            pause_next,
            requested,
            respond,
        })
    }

    async fn assign(
        self,
        owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
        command_type: u32,
        payload: Vec<u8>,
    ) -> TestResult {
        use std::time::Duration;
        tokio::time::timeout(Duration::from_secs(5), self.requested.notified()).await?;
        let commands = [(command_type, payload)];
        let mut wire = std::pin::pin!(super::parameter_spell_fixtures::exchange(
            owner, admitted, &commands, false, false,
        ));
        eprintln!(
            "POSTGRES_STARTED quick_loot_food_to_gold_pouch_waits_for_relog_snapshot_postgres"
        );
        // A bounded negative assertion: hold the actual response while the old
        // join can complete and the client can attempt its first assignment.
        if let Ok(early) = tokio::time::timeout(Duration::from_secs(2), &mut wire).await {
            return Err(format!(
                "first assignment completed before Premium arrived: {:?}",
                early?
            )
            .into());
        }
        self.respond.notify_one();
        let (replies, _) = wire.await?;
        assert_eq!(replies, [(CommandStatus::Accepted, Vec::new())]);
        Ok(())
    }
}

async fn logout_once(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use super::item_look_fixtures::read_frame;
    use crate::foundation::GameSessionState;
    use crate::gameplay_transport::composed_admission::{
        CONDITION_RELEASE_PROBE, HolderCheckoutProbe,
    };
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::{
        ClientCommandValue, MessageType, decode_command_result, decode_wire_envelope,
        encode_client_command,
        logout::{COMMAND_TYPE_LOGOUT_INTENT, decode_logout_result},
    };
    use std::{cell::Cell, sync::Arc, time::Duration};
    use tokio::io::AsyncWriteExt as _;

    let probe = Arc::new(HolderCheckoutProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let drive = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let command_id = admitted.continuity.next_command_id;
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_LOGOUT_INTENT,
                payload: &[],
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        probe.contend(&harness.root).await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(envelope.payload())?;
                assert_eq!(reply.command_id, command_id);
                assert_eq!(
                    decode_logout_result(reply.payload).map_err(debug)?,
                    LogoutResult::Accepted
                );
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        Duration::from_secs(30),
        super::fixture::join_two(
            CONDITION_RELEASE_PROBE.scope(
                Cell::new(Some(Arc::clone(&probe))),
                serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS),
            ),
            drive,
        ),
    )
    .await?;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
    let session = admitted.game_session_id;
    assert_eq!(
        FreshAdmissionStore::from_root(harness.root.clone())
            .current_session(session)
            .await?
            .session_state(),
        GameSessionState::Terminal
    );
    let actor = admitted.runtime_actor.ok_or("readmitted actor")?;
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(actor, session)
            .is_err()
    );
    Ok(())
}
