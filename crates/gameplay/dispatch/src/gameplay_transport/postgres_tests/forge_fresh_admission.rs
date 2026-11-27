//! A freshly admitted hunter uses the Premium snapshot that the account owner pulled.
use super::fixture::{Harness, debug, id};
use super::house_transfer_fixtures::RecoverySource;
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult};
use crate::domain::forge_creature::ForgeCreature;
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelContentPin, ChannelRuntimeV1, CharacterId, GameSessionId,
    MovementLocalPosition, NodeId, WorldId,
};
use crate::gameplay_transport::{
    FreshAdmissionAttempt,
    connection::{FirstEntryOutcome, FreshAdmissionAuthority},
    kill_reward::DrainOutcome,
    spell_premium_coordinator::{
        PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
    },
    world_spatial::VisibleKind,
};
use crate::premium::{
    PremiumConsumer, TrustedNow,
    client::{PremiumClientConfig, PremiumSnapshotClient},
    refresh::PremiumRefresher,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, FightMode, FightModes};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::{Arc, atomic::Ordering};

#[allow(
    dead_code,
    reason = "the shared Premium producer also serves the social test suite"
)]
#[path = "../../../../social/src/premium/test_producer.rs"]
mod premium_test_producer;

#[test]
fn fresh_premium_admission_credits_fiendish_kill_dust() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_async(async |admin| hunt(admin, ForgeCreature::Fiendish).await)
}

#[test]
fn influenced_kill_mints_and_publishes_the_corpse_even_when_loot_misses() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let kind = ForgeCreature::new_influenced(3).ok_or("influenced classification")?;
    run_async(async move |admin| hunt(admin, kind).await)
}

struct PremiumClock;
impl clock_registration::Registered for PremiumClock {}
impl RegisteredPremiumClock for PremiumClock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        TrustedNow::new(1_790_769_660_000_000, 0)
    }
}

async fn hunt(admin: String, kind: ForgeCreature) -> TestResult {
    use premium_test_producer::{Reply, TestPki, TestProducer, snapshot};
    let fiendish = kind == ForgeCreature::Fiendish;
    let tag = if fiendish {
        "fresh_fiendish"
    } else {
        "fresh_influenced"
    };
    let h = Harness::create_with_admission_seed(admin, tag, true, false).await?;
    let result: TestResult = async {
        let source = RecoverySource::start(&h).await?;
        let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool).await?;
        use crate::foundation::admission_authority_publication::{
            AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
        };
        let mut runtime_source = super::fresh_fixture::fresh::Source::new(now)?;
        runtime_source.rows.retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
        h.root.publish_runtime_readiness(
            &h.node,
            &AdmissionAuthorityPublicationV1::prepare(&runtime_source, now).map_err(debug)?,
        ).await.map_err(debug)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let loot = (!fiendish).then(super::forge_creature_fixtures::rat_loot_that_misses).transpose()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_custom_loot_tables(
                world,
                Some(include_bytes!("../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json")),
                loot.as_deref(),
            );
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let room = pin.qualified_room().clone();
        let start = MovementLocalPosition { x: 2, y: 11, floor: -7 };
        let death_tile = MovementLocalPosition { x: 1, ..start };
        let channel_pin = ChannelContentPin::from_activation(
            world, pin.activation_sequence(), pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(), room.frame_binding().digest(),
            room.map_revision_digest(), (start.x, start.y, start.floor),
        );
        let mut deps = Dependencies::with_content_and_capacity(
            channel_pin.clone(), room.clone(), native.spell_book().clone(), 3,
        )?;
        // The generic gameplay fixture reserves a synthetic session. This case
        // starts with an empty Channel and creates its actor only through ADMIT.
        *deps.runtime.get_mut() = ChannelRuntimeV1::from_committed_assignment(
            world, deps.admitted.channel_id, NodeId::decode(&id(1)).map_err(debug)?,
            1, 1, 1, "runtime-scope-assignment:1", 3, channel_pin.clone(),
        ).map_err(debug)?;
        deps.combat_room = Some(room.clone());
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &source.evidence;
        owner.active_generation = Some(active);
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;

        let pki = TestPki::new();
        let producer = TestProducer::start(&pki, |request| Reply::json(snapshot(request, 7, &[]))).await;
        let client = PremiumSnapshotClient::new(&PremiumClientConfig {
            origin: producer.origin.clone(),
            identity_pem: pki.client_identity_pem.into_bytes(),
            platform_ca_pem: pki.ca_pem.into_bytes(),
        })?;
        let consumer = Arc::new(PremiumConsumer::default());
        let coordinator = SpellPremiumCoordinator::new(
            consumer.clone(), Some(Arc::new(PremiumClock)),
            Some(PremiumActivation {
                activation_id: "fresh-forge-premium".into(),
                switch_over_us: 1_790_769_600_000_000,
            }),
            Some(h.root.clone()),
        );
        owner.premium = PremiumRefresher::new(consumer, h.root.clone(), Some(client));
        owner.premium_coordinator = Some(&coordinator);
        let session = GameSessionId::decode(&id(91)).map_err(debug)?;
        let token = RecoverySource::fresh_token(91)?;
        let admitted = owner.admit(FreshAdmissionAttempt {
            character_id: CharacterId::decode(&id(41)).map_err(debug)?,
            admission_material: token.as_bytes(),
            game_session_id: session,
            transport: AuthenticatedTransportRefV1::decode(&[91; 16]).map_err(debug)?,
            supported_capabilities: &[
                4, 6, 12, 14, oteryn_protocol_oteryn::item_forge::CAPABILITY_ITEM_FORGE_V1,
            ],
        }).await.map_err(debug)?;
        assert_eq!(admitted.game_session_id, session);
        assert!(matches!(admitted.first_entry, FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled),
            "fresh hunter must be playable: {:?}", admitted.first_entry);
        let actor = admitted.runtime_actor.ok_or("fresh hunter")?;
        let target = {
            let mut runtime = owner.runtime.lock().await;
            assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), start);
            if fiendish {
            assert!(owner.states.spell_states.lock().await.get(&runtime, actor, session)
                .is_some_and(|state| state.hunting_premium_current(owner.owner_now().get())),
                "the admitted hunter must have Premium before any gameplay tick");
            }
            native.install_companion_policies(&mut runtime).map_err(debug)?;
            let target = runtime.realize_native_qualification_spawn(
                channel_pin, "oteryn:creature.rat", death_tile,
            ).map_err(debug)?.actor;
            runtime.classify_forge_spawn_for_test(target, kind).map_err(debug)?;
            runtime.set_test_creature_health(target, 1).map_err(debug)?;
            let mut attack = owner.states.attack.lock().await;
            assert!(attack.set_modes(&runtime, actor, session, FightModes {
                fight_mode: FightMode::Offensive, ..FightModes::DEFAULT
            }));
            assert_eq!(attack.set_target(&runtime, Some(&room), actor, session, owner.owner_now(),
                Some(EntityRef {
                    identity: target.placement_identity(), generation: target.actor_local_generation(),
                }), 1), AttackIntentDisposition::Ok);
            target
        };
        if fiendish {
        let premium = h.root.load_premium_fence(id(40)).await?.fence.ok_or("durable Premium")?;
        assert_eq!(premium.latest.state, crate::durability::premium_fence::EntitlementState::Active);
        assert_eq!(premium.latest.authority_revision, 7);
        }

        let balance = async |command_id| -> TestResult<i64> {
            let bytes = owner.item_forge_balance(admitted, command_id)
                .await.ok_or("Forge balance")?;
            let json: serde_json::Value = serde_json::from_slice(&bytes)?;
            json["current_dust"].as_i64().ok_or_else(|| "dust balance missing".into())
        };
        assert_eq!(balance(100).await?, 0);
        let before = super::forge_creature_fixtures::ledger(&h.pool).await?;
        assert!(before.is_empty());
        for turn in 1..=64 {
            owner.states.test_owner_time.store(owner.owner_now().get() + 3_000_000, Ordering::Relaxed);
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner.runtime.lock().await.companion_snapshot(target).is_err() {
                break;
            }
            assert!(turn < 64, "the live attack must kill the classified Rat");
        }
        let drained = if fiendish {
            owner.drain_kill_rewards(session).await
        } else {
            drain_with_busy_loot_checkout(&owner, session).await?
        };
        assert_eq!(drained, DrainOutcome::Drained);

        let credits = super::forge_creature_fixtures::ledger(&h.pool).await?;
        if fiendish {
        assert_eq!(credits.len(), 1, "one durable dust credit for this kill");
        assert!(credits[0].4 > 0);
        assert_eq!(balance(101).await?, credits[0].4);
        }

        let corpse_identity = target.corpse_identity();
        let at = {
            let runtime = owner.runtime.lock().await;
            let corpse = runtime.bound_corpse(corpse_identity).ok_or("bound corpse")?;
            assert_eq!(corpse.position, death_tile);
            owner.wire_position_frame(&runtime).and_then(|frame| frame.project(death_tile)).ok_or("wire death tile")?
        };
        let visible = owner.observe_channel_entities(actor).await.ok_or("spatial projection")?;
        assert!(visible.entities.iter().any(|entity| {
            entity.identity == corpse_identity && entity.position == at
                && matches!(entity.kind, VisibleKind::Corpse { .. })
        }));
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_mint_receipts
              WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND materialized_at IS NOT NULL",
        ).fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1, "one durable corpse receipt");
        if fiendish {
            super::forge_creature_fixtures::assert_committed_sliver_in_corpse(&h.pool).await?;
        } else {
            let loot: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries")
                .fetch_one(&h.pool).await?;
            assert_eq!(loot, 0, "a missed loot roll still leaves the corpse");
        }
        eprintln!("POSTGRES_EXECUTED {tag} dust={} corpse_receipts={receipts}", credits.first().map_or(0, |entry| entry.4));
        Ok(())
    }.await;
    let cleanup = h.cleanup().await;
    result?;
    cleanup
}

/// Hold the physical database connection at the ordinary loot read, after
/// the other kill owners have passed their fences.
async fn drain_with_busy_loot_checkout(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult<DrainOutcome> {
    use crate::gameplay_transport::kill_reward::FORGE_LOOT_CHECKOUT;
    use std::future::{Future, poll_fn};
    use std::task::Poll;
    use tokio::sync::Notify;

    let barrier = Arc::new((Notify::new(), Notify::new()));
    let drain = FORGE_LOOT_CHECKOUT.scope(barrier.clone(), owner.drain_kill_rewards(session));
    let mut drain = std::pin::pin!(drain);
    tokio::select! {
        () = barrier.0.notified() => {}
        result = &mut drain => return Err(format!("no ordinary loot checkout: {result:?}").into()),
    }
    let held = owner.root.try_issue_semantic_pass()?;
    barrier.1.notify_one();
    let result = poll_fn(|cx| Poll::Ready(drain.as_mut().poll(cx))).await;
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    match result {
        Poll::Ready(outcome) => Ok(outcome),
        Poll::Pending => Ok(drain.await),
    }
}
