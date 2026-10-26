//! Kill → reward end to end on PostgreSQL: a held auto-attack target is driven by the real
//! `drain_auto_attacks` to a lethal swing, which queues the kill, and the real
//! `drain_kill_rewards` settles it durably (XP row and a linked corpse with its loot) through
//! the composed owner. The only fabricated inputs are the clock (`clock_origin` in the past, then
//! advanced between swings) and the session fixture; the content is the committed production
//! manifest's generation (the production rat: 5 XP, `oteryn:loot.creature.rat`).
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, SessionContinuity,
};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;
use tokio::sync::Mutex;
use {
    crate::foundation::{
        ChannelId, ChannelRuntimeV1, GameSessionId, MovementLocalPosition, NodeId, WorldId,
    },
    oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
};

/// Thalom cells next to each other, neither in a protection zone.
const OPEN_PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const OPEN_RAT: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};

/// One owner swing interval is far below this; every pass of the loop advances the owner clock
/// by it, so each `drain_auto_attacks` finds the next swing due.
const PASS: Duration = Duration::from_secs(3);

#[path = "kill_reward_postgres_fixture/scenario_thread.rs"]
pub(super) mod scenario_thread;

#[test]
fn auto_attack_kill_settles_xp_and_a_linked_corpse_with_loot_on_postgres() -> TestResult {
    scenario_thread::run("kill-reward-e2e", scenario)
}

fn scenario() -> TestResult {
    scenario_with_decay(None)
}

#[test]
fn soulpit_acquisition_real_dispatch_retains_core_prism_and_replay() -> TestResult {
    scenario_thread::run("soulpit-acquisition", || {
        scenario_with_soulpit(None, true, None)
    })
}

#[test]
fn soulpit_acquisition_core_mint_failure_recovers_same_receipts() -> TestResult {
    scenario_thread::run("soulpit-core-recovery", || {
        scenario_with_soulpit(None, true, Some("core"))
    })
}

#[test]
fn soulpit_acquisition_prism_mint_failure_recovers_same_receipts() -> TestResult {
    scenario_thread::run("soulpit-prism-recovery", || {
        scenario_with_soulpit(None, true, Some("prism"))
    })
}

#[test]
fn soulpit_acquisition_stale_scope_cannot_mint_core_or_prism() -> TestResult {
    scenario_thread::run("soulpit-stale-authority", || {
        scenario_with_soulpit(None, true, Some("stale"))
    })
}

pub(super) fn scenario_with_decay(reconcile: Option<bool>) -> TestResult {
    scenario_with_soulpit(reconcile, false, None)
}

fn scenario_with_soulpit(
    reconcile: Option<bool>,
    soulpit: bool,
    recovery: Option<&str>,
) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "kill_reward_e2e", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // The production generation, activated for the fixture's World on the Thalom source
        // world (its tiles carry the protection-zone flags the swing reads).
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let active = controller.active().expect("activated generation");
        let native = active.native_gameplay().expect("native gameplay state");
        let room = pin.qualified_room().clone();
        // The Thalom entry start is a protection-zone cell, where no swing is legal: enter the
        // Channel on the open cell beside it instead (as the owner auto-attack tests do).
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (OPEN_PLAYER.x, OPEN_PLAYER.y, OPEN_PLAYER.floor),
        );

        // ATTACK-2: the swing weapon is read from the equipment owner, which checks the
        // Channel's durably active Content against the pin (the activation `serve` records) and
        // needs its empty seed (as fresh admission leaves it; no item worn, so fists).
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), NULL, $4, $5, $6)")
            .bind(id(42).as_slice())
            .bind(id(43).as_slice())
            .bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice())
            .execute(&harness.pool)
            .await?;

        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            channel_pin.clone(),
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        // A2: the admitted session's Character lease bound to its slot, as first entry does; the
        // swing's attributed damage commit reads it.
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    // The fresh session's current Character lease generation.
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(channel_pin, "oteryn:creature.rat", OPEN_RAT)
            .map_err(debug)?;
        if soulpit {
            let exact = runtime
                .visible_entities()
                .creatures
                .first()
                .ok_or("rat")?
                .actor;
            // Pick a semantic clock fixture for two successful reference rolls, then let the
            // actual runtime classifier install the mark. No client controls these facts.
            let mut chosen = None;
            for now in 0..100_000_u64 {
                use sha2::{Digest, Sha256};
                let until = now + 3_600_000_000;
                let digest = Sha256::new()
                    .chain_update(b"oteryn:soulpit.acquisition:v1\0")
                    .chain_update(exact.placement_identity())
                    .chain_update(until.to_be_bytes())
                    .finalize();
                let mut draws = [0; 4];
                for (draw, bytes) in draws.iter_mut().zip(digest.as_chunks::<8>().0) {
                    let mut word = [0; 8];
                    word.copy_from_slice(bytes);
                    *draw = u64::from_be_bytes(word);
                }
                let drops = oteryn_gameplay_items::soul_core::soulpit_acquisition(
                    native,
                    "oteryn:creature.rat",
                    "definition-r1",
                    draws,
                );
                if drops.as_ref().is_some_and(|d| {
                    d.len() == 2 && d[0].production_key == "oteryn:item.tibia.i47398"
                }) {
                    chosen = Some(now);
                    break;
                }
            }
            runtime
                .refresh_fiendish_creatures(
                    chosen.ok_or("bounded clock fixture finds core and prism")?,
                    |_| true,
                )
                .map_err(debug)?;
            assert!(
                runtime
                    .companion_snapshot(exact)
                    .map_err(debug)?
                    .state
                    .fiendish_until_us
                    .is_some()
            );
        }
        let rat = {
            let creature = runtime
                .visible_entities()
                .creatures
                .first()
                .copied()
                .expect("the rat is visible");
            EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };

        let door_content =
            crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                .map_err(debug)?;
        let fence = super::fixture::fence(1)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            door_content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        let evidence =
            crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
        let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
        let achievements =
            crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
        let charms =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let mut admitted = AdmittedSession {
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
        };
        let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
            root: &harness.root,
            character: &authority,
            holder: &harness.node,
            evidence: &evidence,
            world_id: world,
            channel_id: channel,
            // Gameplay-only fixture: it never admits through this owner, so the ADMIT0 binding
            // declares nothing and only has to be a valid Channel boot binding.
            required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .expect("an empty declared set binds"),
            runtime: &runtime,
            door: &door,
            spells: &spells,
            achievements: &achievements,
            imported_charms: &charms,
            room: &room,
        }.build();
        owner.active_generation = Some(active);
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        // In the past, so the owner clock already stands well after the swing clock's origin.
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(60))
            .expect("a monotonic clock older than a minute");
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            // Level 50 keeps minimum fist damage above the rat's defence and
                            // armor, independent of the content-digest-seeded RNG stream.
                            level: 50,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }

        // ARCH-PROGRESSION-SOURCE-0 §1.5: the session's progression binding, formed by the
        // production admission step from the Character root's stored revisions.
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner.progression_binding(session).is_some(),
            "the fresh Character's session binds the generation's progression"
        );
        assert_eq!(xp_receipts(&harness).await?, 0);

        // The target is held (attacking state), then the real owner pass swings until the rat
        // dies; each pass is one swing interval later on the owner clock.
        {
            let runtime = owner.runtime.lock().await;
            let now = owner.owner_now();
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    now,
                    Some(rat),
                    1
                ),
                AttackIntentDisposition::Ok
            );
        }
        let mut swings = 0;
        loop {
            // ATTACK-2: the swing weapon (here fists) is read from the equipment owner first.
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session)
            {
                break;
            }
            swings += 1;
            assert!(swings < 64, "the held rat dies to auto-attack swings");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(PASS)
                .expect("a monotonic clock older than the passes");
        }
        // The kill is queued, not yet settled: no XP row exists before `drain_kill_rewards`.
        assert_eq!(xp_receipts(&harness).await?, 0);
        assert_eq!(corpse_receipts(&harness).await?, 0);

        if soulpit {
            // Rebuild a lost queue entry from the retained lethal projection after the live
            // mark expires. Its original death time, classification and outcome must survive.
            let (facts, exact, observed, activity) = {
                let attack = owner.states.attack.lock().await;
                let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                    attack.kills().take(session, 2)
                else {
                    return Err("captured soulpit kill".into());
                };
                let entry = taken.entry().ok_or("captured entry")?;
                let saved = (
                    entry.facts,
                    entry.target,
                    entry.activity_observed_at,
                    entry.activity_at_death.clone().unwrap_or_default(),
                );
                taken.finish();
                saved
            };
            let mut runtime = owner.runtime.lock().await;
            let until = runtime
                .companion_snapshot_including_dead(exact)
                .map_err(debug)?
                .state
                .fiendish_until_us
                .ok_or("fiendish expiry")?;
            let attack = owner.states.attack.lock().await;
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                crate::gameplay_transport::kill_reward::KillSink {
                    native: Some(native),
                    activity: &activity,
                    queue: attack.kills(),
                    rewards: native.reward_table(),
                },
                exact,
                facts.principal,
                until / 1000 + 1,
                observed,
            );
            assert!(attack.kills().has_session(session));
        }
        if let Some(recovery) = recovery {
            if recovery == "stale" {
                // Replace only live scope generation; the captured death and principal stay valid.
                let mut tx = harness.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE game_runtime_scope_assignments SET ownership_generation=ownership_generation+1 WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                    .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                let _outcome = owner.drain_kill_rewards(session).await;
                let items: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key IN ('oteryn:item.tibia.i47398','oteryn:item.tibia.i49164')").fetch_one(&harness.pool).await?;
                assert_eq!(items, 0);
                assert_eq!(corpse_receipts(&harness).await?, 0);
                assert_eq!(xp_receipts(&harness).await?, 0);
                drop(owner);
                drop(authority);
                drop(seal);
                return harness.cleanup().await;
            }
            let trigger = match recovery {
                "core" => {
                    "CREATE TRIGGER soulpit_injected_mint BEFORE INSERT ON game_item_instances FOR EACH ROW WHEN (NEW.definition_production_key='oteryn:item.tibia.i47398') EXECUTE FUNCTION soulpit_injected_mint()"
                }
                "prism" => {
                    "CREATE TRIGGER soulpit_injected_mint BEFORE INSERT ON game_item_instances FOR EACH ROW WHEN (NEW.definition_production_key='oteryn:item.tibia.i49164') EXECUTE FUNCTION soulpit_injected_mint()"
                }
                _ => return Err("unknown recovery fixture".into()),
            };
            sqlx::raw_sql("CREATE SEQUENCE soulpit_mint_attempt; CREATE FUNCTION soulpit_injected_mint() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('soulpit_mint_attempt'); RAISE EXCEPTION 'soulpit injected acquisition failure'; END $$").execute(&harness.pool).await?;
            sqlx::raw_sql(trigger).execute(&harness.pool).await?;
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Deferred
            );
            assert!(
                sqlx::query_scalar::<_, bool>("SELECT is_called FROM soulpit_mint_attempt")
                    .fetch_one(&harness.pool)
                    .await?,
                "failure reached selected core/prism mint"
            );
            let failed_key = if recovery == "core" {
                "oteryn:item.tibia.i47398"
            } else {
                "oteryn:item.tibia.i49164"
            };
            let failed: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1",
            )
            .bind(failed_key)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(failed, 0, "failed mint has no durable instance");
            let reserved: Vec<String> = sqlx::query_scalar("SELECT transaction_id::text FROM game_item_mint_reservations ORDER BY transaction_id").fetch_all(&harness.pool).await?;
            assert!(!reserved.is_empty());
            assert_eq!(xp_receipts(&harness).await?, 1);
            assert_eq!(corpse_receipts(&harness).await?, 1);
            assert!(
                owner
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .has_session(session)
            );
            sqlx::raw_sql("DROP TRIGGER soulpit_injected_mint ON game_item_instances; DROP FUNCTION soulpit_injected_mint(); DROP SEQUENCE soulpit_mint_attempt").execute(&harness.pool).await?;
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            let reused: Vec<String> = sqlx::query_scalar("SELECT transaction_id::text FROM game_item_mint_reservations ORDER BY transaction_id").fetch_all(&harness.pool).await?;
            assert!(
                reserved.iter().all(|id| reused.contains(id)),
                "recovery keeps every frozen transaction identity"
            );
            let committed: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_mint_receipts WHERE transaction_id::text=ANY($1)",
            )
            .bind(&reserved)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                usize::try_from(committed)?,
                reserved.len(),
                "every pre-failure reservation commits under its original transaction"
            );
        }
        let mut drains = 0;
        while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
            drains += 1;
            assert!(drains < 8, "the queued kill settles");
        }
        assert!(
            !owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session),
            "the settled kill leaves the queue"
        );

        // XP: exactly one committed receipt, the production rat's 5 XP on the fenced Character.
        assert_eq!(xp_receipts(&harness).await?, 1);
        let experience: i64 =
            sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(experience, 5);
        // Corpse: one materialized corpse minted for this Character's kill, and its loot rows
        // are children of that very corpse.
        assert_eq!(corpse_receipts(&harness).await?, 1);
        let children: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_corpse_container_entries e \
             JOIN game_item_mint_receipts r ON r.item_instance_id = e.parent_item_instance_id",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert!(children >= 1, "the corpse carries the rat's loot");

        if soulpit {
            let drops: Vec<String> = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key IN ('oteryn:item.tibia.i47398','oteryn:item.tibia.i49164') ORDER BY i.definition_production_key")
                .fetch_all(&harness.pool).await?;
            assert_eq!(
                drops,
                ["oteryn:item.tibia.i47398", "oteryn:item.tibia.i49164"]
            );
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key IN ('oteryn:item.tibia.i47398','oteryn:item.tibia.i49164')")
                .fetch_one(&harness.pool).await?;
            assert_eq!(retained, 2);
            assert_eq!(xp_receipts(&harness).await?, 1);
            assert_eq!(corpse_receipts(&harness).await?, 1);
        }
        let decay = match reconcile {
            Some(reconcile) => {
                super::corpse_decay::qualify(&mut owner, &harness, children, reconcile)
                    .await
                    .map_err(|error| format!("corpse decay qualification: {error}").into())
            }
            None => Ok(()),
        };

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        decay
    })
}

async fn xp_receipts(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
            .fetch_one(&harness.pool)
            .await?,
    )
}

async fn corpse_receipts(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key = $1 \
         AND materialized_at IS NOT NULL",
    )
    .bind(crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY)
    .fetch_one(&harness.pool)
    .await?)
}
