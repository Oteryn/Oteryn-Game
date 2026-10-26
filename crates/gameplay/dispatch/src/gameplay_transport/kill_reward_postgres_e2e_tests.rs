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

#[path = "kill_reward_postgres_fixture/summer_settlement.rs"]
mod summer_settlement;
use summer_settlement::qualify_summer_settlement;

#[test]
fn auto_attack_kill_settles_xp_and_a_linked_corpse_with_loot_on_postgres() -> TestResult {
    scenario_thread::run("kill-reward-e2e", scenario)
}

fn scenario() -> TestResult {
    scenario_with_decay(None)
}

pub(super) fn scenario_with_decay(reconcile: Option<bool>) -> TestResult {
    scenario_with_capacity(reconcile, false)
}

pub(super) fn scenario_with_capacity(reconcile: Option<bool>, capacity: bool) -> TestResult {
    scenario_with_creature(reconcile, capacity, None, false)
}

#[test]
fn summer_radiant_all_six_execute_attacks_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-radiant-attacks", || {
        for name in [
            "acolyte",
            "inquisitor",
            "paragon",
            "templar",
            "warden",
            "zealot",
        ] {
            scenario_with_creature(None, false, Some(name), false)?;
        }
        Ok(())
    })
}

#[test]
fn summer_radiant_all_six_kills_settle_and_decay_on_postgres() -> TestResult {
    scenario_thread::run("summer-radiant-settlements", || {
        for name in [
            "acolyte",
            "inquisitor",
            "paragon",
            "templar",
            "warden",
            "zealot",
        ] {
            scenario_with_creature(None, false, Some(name), true)?;
        }
        Ok(())
    })
}

#[test]
fn summer_corpse_stages_recover_under_a_new_process_and_owner_generation() -> TestResult {
    scenario_thread::run("summer-corpse-handoff", || {
        scenario_with_creature(Some(true), false, Some("warden"), true)
    })
}

fn scenario_with_creature(
    reconcile: Option<bool>,
    capacity: bool,
    radiant: Option<&str>,
    settle_radiant: bool,
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
        let creature_key = radiant.map_or_else(
            || "oteryn:creature.rat".to_owned(),
            |name| format!("oteryn:creature.radiant_{name}"),
        );
        let expected_xp = if settle_radiant {
            native
                .reward_table()
                .row(&creature_key)
                .map_err(debug)?
                .xp_amount
                .get()
        } else {
            5
        };
        if settle_radiant {
            let exact = match radiant {
                Some("acolyte") => 32_960,
                Some("inquisitor") => 32_460,
                Some("paragon") => 34_780,
                Some("templar") => 34_520,
                Some("warden") => 35_540,
                Some("zealot") => 35_220,
                _ => return Err("unknown Summer Radiant".into()),
            };
            assert_eq!(expected_xp, exact, "{creature_key} exact native XP");
        }
        if settle_radiant {
            // Select a positive real occurrence without changing a drop probability.
            // Roll back unused player reservations to advance the reusable creature
            // slot's generation (player slot 1, creature slot 2), then spawn and kill that actual native creature.
            let reward = native.reward_table().row(&creature_key).map_err(debug)?;
            let mut generation = None;
            for candidate in 1..=20_000 {
                let death = oteryn_gameplay_items::loot_plan::LootPlanDeathKey::new(
                    id(42),
                    id(43),
                    1,
                    2,
                    candidate,
                );
                let plan = oteryn_gameplay_items::loot_plan::plan_creature_loot(
                    death,
                    &reward.loot_table_ref,
                    &reward.loot_table,
                )?;
                if !plan.entries.is_empty() {
                    generation = Some(candidate);
                    break;
                }
            }
            for _ in 1..generation.ok_or("positive bounded native loot occurrence")? {
                let unused = runtime
                    .reserve_fresh_session(GameSessionId::decode(&id(245)).map_err(debug)?)
                    .map_err(debug)?;
                runtime
                    .rollback_definitely_uncommitted(unused)
                    .map_err(debug)?;
            }
        }
        runtime
            .realize_native_qualification_spawn(channel_pin, &creature_key, OPEN_RAT)
            .map_err(debug)?;
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
                            // Level 50 overcomes the rat's defence and armor. A high-level
                            // attacker kills the real Radiant in bounded swings;
                            // its native HP, defence, loot and corpse facts stay unchanged.
                            level: if settle_radiant { 1_000_000 } else { 50 },
                            max_health: if radiant.is_some() { 1_000_000 } else { 185 },
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }

        if radiant.is_some() && !settle_radiant {
            let initial = {
                let runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, actor, session)
                    .ok_or("player spell state")?
                    .vitals()
                    .health
            };
            let mut damaged = false;
            for _ in 0..32 {
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let state = states
                    .get(&runtime, actor, session)
                    .ok_or("player spell state")?;
                if state.vitals().health < initial {
                    damaged = true;
                    // A repeated owner call inside the same think window cannot apply the
                    // source attack twice. All content and attack schedules are unmodified.
                    let after = state.vitals();
                    drop(states);
                    drop(runtime);
                    owner.drain_monster_melee().await;
                    let runtime = owner.runtime.lock().await;
                    assert_eq!(
                        owner
                            .states
                            .spell_states
                            .lock()
                            .await
                            .get(&runtime, actor, session)
                            .ok_or("player spell state")?
                            .vitals(),
                        after
                    );
                    break;
                }
                drop(states);
                drop(runtime);
                owner.clock_origin = owner.clock_origin.checked_sub(PASS).ok_or("attack clock")?;
            }
            assert!(
                damaged,
                "{creature_key} must execute a real damaging attack"
            );
            assert_eq!(xp_receipts(&harness).await?, 0);
            assert_eq!(corpse_receipts(&harness).await?, 0);
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
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
            let queued = owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session);
            if queued {
                break;
            }
            swings += 1;
            assert!(swings < 64, "{creature_key} dies to auto-attack swings");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(PASS)
                .expect("a monotonic clock older than the passes");
        }
        // No durable rewards exist before the reward drain.
        assert_eq!(xp_receipts(&harness).await?, 0);
        assert_eq!(corpse_receipts(&harness).await?, 0);

        let replay = if settle_radiant {
            let attack = owner.states.attack.lock().await;
            let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                attack.kills().take(session, 2)
            else {
                return Err("queued real kill".into());
            };
            let entry = taken.entry().ok_or("captured kill")?;
            let replay = (entry.facts, entry.target, entry.activity_observed_at);
            taken.requeue();
            Some(replay)
        } else {
            None
        };
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

        // XP: exactly one committed receipt for the native amount on the fenced Character.
        assert_eq!(xp_receipts(&harness).await?, 1);
        let experience: i64 =
            sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(experience, expected_xp);
        // Corpse: one materialized corpse minted for this Character's kill, and its loot rows
        // are children of that very corpse.
        assert_eq!(corpse_receipts(&harness).await?, 1);
        let children: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_corpse_container_entries e \
             JOIN game_item_mint_receipts r ON r.item_instance_id = e.parent_item_instance_id",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert!(
            children >= 1,
            "{creature_key} commits a positive native loot plan"
        );

        if let Some((facts, target, observed)) = replay {
            {
                let mut runtime = owner.runtime.lock().await;
                let attack = owner.states.attack.lock().await;
                crate::gameplay_transport::kill_reward::record_creature_kill(
                    &mut runtime,
                    crate::gameplay_transport::kill_reward::KillSink {
                        activity: &[],
                        queue: attack.kills(),
                        rewards: native.reward_table(),
                    },
                    target,
                    facts.principal,
                    facts.death_at_ms,
                    observed,
                );
                assert!(
                    attack.kills().has_session(session),
                    "the same committed kill is replayed through the router"
                );
            }
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            assert_eq!(xp_receipts(&harness).await?, 1);
            assert_eq!(corpse_receipts(&harness).await?, 1);
            let replay_children: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                replay_children, children,
                "kill replay never duplicates loot"
            );
        }
        if settle_radiant {
            qualify_summer_settlement(
                &mut owner,
                &harness,
                &creature_key,
                expected_xp,
                children,
                reconcile == Some(true),
                replay
                    .map(|(_, target, _)| target)
                    .ok_or("real captured kill")?,
            )
            .await?;
        }
        let children = if capacity {
            super::summer_corpse_capacity::qualify(&mut owner, &harness).await?
        } else {
            children
        };
        let decay = match reconcile.filter(|_| !settle_radiant) {
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
