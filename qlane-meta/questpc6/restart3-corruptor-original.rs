//! Newhaven Corruptor gear travels through the actual kill, corpse, take and equip owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, TakeOutcome};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::tutorial::{TutorialEvent, corruptor_items};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::item_view::ItemMoveOutcome;
use oteryn_protocol_oteryn::quick_loot::{QuickLootAction, QuickLootRequest};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::{Duration, Instant};

#[test]
fn newhaven_corruptor_all_five_vocations_kill_loot_take_equip() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("newhaven-corruptor", || {
        for vocation in ["knight", "paladin", "sorcerer", "druid", "monk"] {
            let started = Instant::now();
            scenario(vocation, false, false, vocation == "knight", false)?;
            println!(
                "NEWHAVEN_{vocation} POSTGRES_CASE_COMPLETED {:?}",
                started.elapsed()
            );
        }
        Ok(())
    })
}

#[test]
fn newhaven_corruptor_low_stamina_and_double_loot_keep_one_guaranteed_set() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("newhaven-corruptor-modifiers", || {
        let started = Instant::now();
        scenario("monk", true, false, false, false)?;
        scenario("paladin", false, true, false, false)?;
        println!(
            "NEWHAVEN_MODIFIERS POSTGRES_CASE_COMPLETED {:?}",
            started.elapsed()
        );
        Ok(())
    })
}

#[test]
fn newhaven_corruptor_lifecycle_recovers_original_corpse_and_child_custody() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("newhaven-corruptor-lifecycle", || {
        let started = Instant::now();
        scenario("knight", false, false, false, true)?;
        println!(
            "NEWHAVEN_LIFECYCLE POSTGRES_CASE_COMPLETED {:?}",
            started.elapsed()
        );
        Ok(())
    })
}

fn scenario(
    vocation: &'static str,
    low_stamina: bool,
    double_loot: bool,
    fail_commit: bool,
    lifecycle: bool,
) -> TestResult {
    use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
    let schedule = Schedule::parse(if double_loot {
        r#"{"events":[{"name":"newhaven-double","startdate":"01/01/2030","enddate":"01/02/2030","ingame":{"lootrate":200}}]}"#
    } else {r#"{"events":[]}"#}).map_err(debug)?;
    let _clock = test_clock::set(schedule, 1_893_456_000);

    run(async move |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "newhaven_corruptor", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_source(
            world,
            "content-1",
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        )?;
        let active = controller.active().ok_or("native generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        native
            .reward_table()
            .row("oteryn:creature.the_corruptor")
            .map_err(|reason| {
                format!(
                    "Newhaven production kill reward admission: {}",
                    reason.as_str()
                )
            })?;
        let room = pin.qualified_room().clone();
        super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (32534, 32513, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin.clone(),
            room.clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let target = {
            let mut runtime = deps.runtime.lock().await;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        x: 2,
                        y: 11,
                        floor: -7,
                    },
                )
                .map_err(debug)?;
            if fail_commit || double_loot {
                // Pick an actual actor occurrence with an ordinary Brass Armor
                // roll. Probabilities stay untouched; the special set must not
                // replace that independently caused output on the retry. The
                // event case selects a base gold miss that200% loot restores.
                // Knight also rolls a real mushroom for the native take route.
                let reward = native
                    .reward_table()
                    .row("oteryn:creature.the_corruptor")
                    .map_err(debug)?;
                let mut witness = None;
                for generation in 1..=20_000 {
                    let death = oteryn_gameplay_items::loot_plan::LootPlanDeathKey::new(
                        id(42),
                        id(43),
                        1,
                        2,
                        generation,
                    );
                    let plan = oteryn_gameplay_items::loot_plan::plan_creature_loot(
                        death,
                        &reward.loot_table_ref,
                        &reward.loot_table,
                    )
                    .map_err(debug)?;
                    let selected = if double_loot {
                        "oteryn:item.tibia.i3031"
                    } else {
                        "oteryn:item.tibia.i3359"
                    };
                    let present = plan
                        .entries
                        .iter()
                        .any(|entry| entry.item.production_key == selected);
                    let mushroom = plan
                        .entries
                        .iter()
                        .any(|entry| entry.item.production_key == "oteryn:item.tibia.i3723");
                    if present != double_loot && (!fail_commit || mushroom) {
                        witness = Some(generation);
                        break;
                    }
                }
                for _ in 1..witness.ok_or("ordinary Corruptor occurrence")? {
                    let unused = runtime
                        .reserve_fresh_session(GameSessionId::decode(&id(245)).map_err(debug)?)
                        .map_err(debug)?;
                    runtime
                        .rollback_definitely_uncommitted(unused)
                        .map_err(debug)?;
                }
            }
            runtime
                .realize_native_qualification_spawn(
                    channel_pin,
                    "oteryn:creature.the_corruptor",
                    MovementLocalPosition {
                        x: 1,
                        y: 11,
                        floor: -7,
                    },
                )
                .map_err(debug)?;
            let creature = runtime
                .visible_entities()
                .creatures
                .first()
                .copied()
                .ok_or("Corruptor spawn")?;
            EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.clock_origin = Instant::now()
            .checked_sub(Duration::from_secs(60))
            .ok_or("clock")?;
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
                            level: 200,
                            max_health: 3000,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now()
                    )
                    .is_some()
            );
        }
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        // Newhaven weapon rewards retain their actual level-six restrictions.
        // This character seed represents an eligible tutorial participant.
        {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_character_progression_state SET level=6,total_experience=1500 WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let current =
            crate::gameplay_transport::character_gameplay_fence::current(&owner, session, false)
                .await
                .ok_or("gameplay fence")?;
        let formula = native.training_formula().ok_or("training formula")?;
        let choice = crate::gameplay_transport::vocation_choice::vocation_choice_request(
            crate::durability::character_build::BuildOccurrence::from_bytes(id(94))
                .map_err(debug)?,
            crate::durability::character_build::DurableBuildState::default(),
            vocation,
            formula,
            None,
        )
        .map_err(debug)?;
        owner
            .revision_sequencer
            .acquire(current.character_id)
            .await
            .commit_build(
                owner.root,
                owner.character,
                owner.holder,
                current,
                choice,
                formula,
                None,
            )
            .await
            .map_err(debug)?;
        let current =
            crate::gameplay_transport::character_gameplay_fence::current(&owner, session, false)
                .await
                .ok_or("vocation fence")?;
        h.root
            .initialize_familiar_group(&authority, &h.node, current)
            .await
            .map_err(|error| format!("Newhaven ordinary group admission: {error:?}"))?;
        let armed = owner
            .tutorial_event(current, TutorialEvent::Forcefield)
            .await
            .map_err(|_| "arm Corruptor tutorial")?
            .ok_or("forcefield arms reward")?;
        assert!(armed.progress.corruptor);
        if low_stamina || fail_commit {
            let runtime = owner.runtime.lock().await;
            owner.states.spell_states.lock().await.load_stamina(
                &runtime,
                actor,
                session,
                if fail_commit { 841 } else { 0 },
            );
        }
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        // Only the empty backpack is a fixture. Every reward below must come from the kill.
        {
            let index = native.item_index().ok_or("item index")?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(
                crate::durability::item_fee_burn::test_support::item(
                    210,
                    "oteryn:item.tibia.i2854",
                    index
                        .revision("oteryn:item.tibia.i2854")
                        .ok_or("backpack revision")?,
                    1,
                ),
            ))
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    owner.owner_now(),
                    Some(target),
                    1
                ),
                AttackIntentDisposition::Ok
            );
        }
        let mut swings = 0;
        while !owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(session)
        {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            swings += 1;
            assert!(swings < 1024, "Corruptor dies to actual auto-attacks");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(3))
                .ok_or("attack clock")?;
        }
        let (facts, killed_target, observed, frozen_table) = {
            let attacks = owner.states.attack.lock().await;
            let TakeOutcome::Taken(taken) = attacks.kills().take(session, 2) else {
                return Err("actual Corruptor kill queued".into());
            };
            let entry = taken.entry().ok_or("Corruptor death facts")?;
            let replay = (
                entry.facts,
                entry.target,
                entry.activity_observed_at,
                std::sync::Arc::clone(&entry.tutorial_loot),
            );
            taken.requeue();
            replay
        };
        let prepared = h
            .root
            .prepare_tutorial_corruptor(&authority, &h.node, current, facts.death)
            .await
            .map_err(debug)?
            .ok_or("prepared tutorial gear")?;
        assert_eq!(prepared.items(), corruptor_items(vocation));
        assert!(
            owner
                .tutorial_progress(current)
                .await
                .map_err(|_| "retained arm")?
                .ok_or("armed progress")?
                .progress
                .corruptor,
            "preparation never consumes the reward"
        );
        if fail_commit {
            sqlx::raw_sql(sqlx::AssertSqlSafe(
                "CREATE FUNCTION newhaven_reject_claim() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'newhaven simulated commit failure'; END $$; CREATE TRIGGER newhaven_reject_claim BEFORE INSERT ON game_tutorial_corruptor_claims FOR EACH ROW EXECUTE FUNCTION newhaven_reject_claim();".to_owned()
            )).execute(&h.pool).await?;
            assert_ne!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            h.root.maintain_ready_once().await.map_err(debug)?;
            let current = crate::gameplay_transport::character_gameplay_fence::current(
                &owner, session, false,
            )
            .await
            .ok_or("failed batch current fence")?;
            assert!(
                owner
                    .tutorial_progress(current)
                    .await
                    .map_err(|_| "failed batch arm")?
                    .ok_or("failed batch progress")?
                    .progress
                    .corruptor
            );
            let committed:i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'")
                .fetch_one(&h.pool).await?;
            assert_eq!(
                committed, 0,
                "corpse and every gear item roll back with the claim"
            );
            sqlx::raw_sql(sqlx::AssertSqlSafe("DROP TRIGGER newhaven_reject_claim ON game_tutorial_corruptor_claims; DROP FUNCTION newhaven_reject_claim();".to_owned()))
                .execute(&h.pool).await?;
            h.root.maintain_ready_once().await.map_err(debug)?;
            let runtime = owner.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .load_stamina(&runtime, actor, session, 840);
            assert!(
                frozen_table.get().is_some(),
                "failed atomic attempt froze ordinary loot"
            );
        }
        drain(&owner, session).await?;
        let corpse: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_mint_receipts WHERE loot_purpose_key=$1 AND materialized_at IS NOT NULL")
            .bind(crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY).fetch_one(&h.pool).await?;
        let corpse: [u8; 16] = corpse.try_into().map_err(|_| "corpse identity")?;
        let expected: Vec<String> = corruptor_items(vocation)
            .iter()
            .map(|id| format!("oteryn:item.tibia.i{id}"))
            .collect();
        let actual: Vec<String> = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal")
            .bind(corpse.as_slice()).fetch_all(&h.pool).await?;
        if low_stamina {
            assert_eq!(
                actual.len(),
                expected.len(),
                "zero stamina suppresses ordinary loot while retaining exactly one guaranteed set"
            );
        }
        if double_loot {
            assert!(
                actual.iter().any(|key| key == "oteryn:item.tibia.i3031"),
                "the real200% event restores the selected ordinary gold miss"
            );
        }
        if fail_commit {
            assert_eq!(
                actual
                    .iter()
                    .filter(|key| key.as_str() == "oteryn:item.tibia.i3359")
                    .count(),
                2,
                "ordinary and guaranteed Brass Armor retain independent causes across failed commit and841->840 retry"
            );
            assert!(
                actual.iter().any(|key| key == "oteryn:item.tibia.i3723"),
                "the selected ordinary mushroom is minted by the actual kill"
            );
        }
        for (id, key) in corruptor_items(vocation).iter().zip(&expected) {
            assert!(actual.contains(key), "{vocation} corpse contains {key}");
            let guaranteed:i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key=$1 AND item_instance_id IN (SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid)")
                .bind(crate::durability::tutorial_corruptor::guarantee_purpose(*id))
                .bind(corpse.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                guaranteed, 1,
                "{vocation} has exactly one special {key}, independently of ordinary drops"
            );
        }
        if lifecycle {
            qualify_lifecycle(&mut owner, &h, corpse).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }
        let mushroom: Option<(Vec<u8>, i64)> = if fail_commit {
            let item: (Vec<u8>, i64) = sqlx::query_as("SELECT uuid_send(i.item_instance_id),i.quantity FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i3723' LIMIT 1")
                .bind(corpse.as_slice()).fetch_one(&h.pool).await?;
            assert!((1..=2).contains(&item.1));
            Some(item)
        } else {
            None
        };
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4, 6, 12, 14, 98, 179, 180],
            )
            .ok_or("Newhaven item command capabilities")?;
        let click = QuickLootRequest {
            action: QuickLootAction::Click,
            corpse: Some(corpse),
            category: None,
            recipient: None,
            recipient_id: None,
            fallback_to_main: true,
            filter_mode: None,
            filter_definition_ref: None,
        };
        let payload =
            oteryn_protocol_oteryn::quick_loot::encode_quick_loot_request(&click).map_err(debug)?;
        let reply = wire_command(
            &owner,
            &mut admitted,
            oteryn_protocol_oteryn::quick_loot::COMMAND_TYPE_QUICK_LOOT,
            &payload,
        )
        .await?;
        let result =
            oteryn_protocol_oteryn::quick_loot::decode_quick_loot_result(&reply).map_err(debug)?;
        assert!(!result.items.is_empty(), "the real corpse is in reach");
        assert!(result.items.iter().all(|item| item.accepted));
        if let Some((item, quantity)) = mushroom {
            let carried: (i64, bool) = sqlx::query_as("SELECT i.quantity,NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries c WHERE c.item_instance_id=i.item_instance_id) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=encode($3,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i3723'")
                .bind(item.as_slice()).bind(id(41).as_slice()).bind(id(210).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                carried,
                (quantity, true),
                "real wire Quick Loot moves the same mushroom and quantity from corpse into the Character backpack"
            );
        }
        for key in &expected {
            let index = native.item_index().ok_or("item index")?;
            let definition = index
                .definition_ref("Item", key, index.revision(key).ok_or("gear revision")?)
                .ok_or("gear definition")?;
            let request = oteryn_protocol_oteryn::hotkey_equip::Request { definition }.encode();
            let reply = wire_command(
                &owner,
                &mut admitted,
                oteryn_protocol_oteryn::hotkey_equip::COMMAND_TYPE_HOTKEY_EQUIP,
                &request,
            )
            .await?;
            assert_eq!(
                oteryn_protocol_oteryn::hotkey_equip::decode_reply(&reply).map_err(debug)?,
                ItemMoveOutcome::Moved,
                "{vocation} equips {key}"
            );
            let count:i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_slots e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2")
                .bind(id(41).as_slice()).bind(key).fetch_one(&h.pool).await?;
            assert_eq!(count, 1, "actual durable equipment contains {key}");
        }
        let original_table = frozen_table.get().cloned().ok_or("frozen ordinary table")?;
        {
            let runtime = owner.runtime.lock().await;
            owner.states.spell_states.lock().await.load_stamina(
                &runtime,
                actor,
                session,
                if low_stamina { 2520 } else { 0 },
            );
        }
        let minted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts")
            .fetch_one(&h.pool)
            .await?;
        {
            let mut runtime = owner.runtime.lock().await;
            let attacks = owner.states.attack.lock().await;
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: attacks.kills(),
                    rewards: native.reward_table(),
                    charms: None,
                },
                killed_target,
                facts.principal,
                facts.death_at_ms,
                observed,
            );
        }
        {
            let attacks = owner.states.attack.lock().await;
            let TakeOutcome::Taken(taken) = attacks.kills().take(session, 2) else {
                return Err("same death replay queued".into());
            };
            let replay = taken.entry().ok_or("replay death")?;
            assert!(
                std::sync::Arc::ptr_eq(&frozen_table, &replay.tutorial_loot),
                "settled corpse recapture shares the original table"
            );
            assert_eq!(replay.tutorial_loot.get(), Some(&original_table));
            taken.requeue();
        }
        drain(&owner, session).await?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_mint_receipts")
                .fetch_one(&h.pool)
                .await?,
            minted,
            "same death replay mints no second gear"
        );
        let current =
            crate::gameplay_transport::character_gameplay_fence::current(&owner, session, false)
                .await
                .ok_or("post-equipment fence")?;
        let progress = owner
            .tutorial_progress(current)
            .await
            .map_err(|_| "read tutorial")?
            .ok_or("tutorial progress")?;
        assert!(!progress.progress.corruptor);
        assert!(progress.progress.hunting);
        assert_eq!(
            progress.version, 2,
            "one atomic consume advances the tutorial once"
        );
        let claims:i64 = sqlx::query_scalar("SELECT count(*) FROM game_tutorial_corruptor_claims WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(claims, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn drain(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult {
    let mut passes = 0;
    while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
        passes += 1;
        assert!(passes < 8, "actual Corruptor settlement drains");
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Ok(())
}

async fn qualify_lifecycle(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    corpse: [u8; 16],
) -> TestResult {
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        include_str!("../../../../../../apps/game-server/migrations/1000_corruptor_lifecycle.sql")
            .to_owned(),
    ))
    .execute(&h.pool)
    .await?;
    let schedules: Vec<(i32, i64, i64)> = sqlx::query_as("SELECT p.source_stage,p.duration_millis,p.expires_at_unix_ms-m.materialized_at FROM game_spell_field_temporal_schedules p JOIN game_item_mint_receipts m USING(item_instance_id) WHERE p.item_instance_id=encode($1,'hex')::uuid ORDER BY source_stage")
        .bind(corpse.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        schedules,
        vec![
            (1, 600_000, 600_000),
            (2, 600_000, 1_200_000),
            (3, 10_000, 1_210_000)
        ]
    );
    let lifetime: i64 = sqlx::query_scalar("SELECT game_corpse_retirement_deadline(item_instance_id)-materialized_at FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(lifetime, 1_810_000);
    for sql in [
        "UPDATE game_spell_field_temporal_schedules SET duration_millis=1 WHERE item_instance_id=encode($1,'hex')::uuid",
        "DELETE FROM game_spell_field_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid",
        "INSERT INTO game_spell_field_temporal_schedules SELECT item_instance_id,4,blocks_movement,blocks_projectile,immovable_block_solid,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest,created_xact_id FROM game_spell_field_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid AND source_stage=1",
    ] {
        let result = sqlx::query(sql)
            .bind(corpse.as_slice())
            .execute(&h.pool)
            .await;
        let Err(error) = result else {
            return Err("immutable corpse schedule accepted mutation".into());
        };
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
    }
    let children: Vec<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY item_instance_id")
        .bind(corpse.as_slice()).fetch_all(&h.pool).await?;
    assert!(
        children.len() >= 3,
        "real Knight guarantee remains in corpse custody"
    );
    owner.drain_source_item_deadlines().await;
    let stages: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_field_temporal_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(stages, 0, "no early first transform");
    owner.drain_corpse_deadlines().await;

    // Translate only the fixture's source clock. Real commit-time clocks,
    // provenance checks, current-scope fences and custody guards remain active.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let delta: i64 = sqlx::query_scalar("SELECT materialized_at-floor(extract(epoch FROM clock_timestamp())*1000)::bigint+1211000 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).bind(delta).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=expires_at_unix_ms-$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).bind(delta).execute(&mut *tx).await?;
    tx.commit().await?;
    restart_lifecycle(h, "transforms")?;
    let state: (String, String, i64) = sqlx::query_as("SELECT definition_production_key,state_revision::text,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, ("oteryn:item.tibia.i52116".into(), "4".into(), 1));
    let retained: Vec<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY item_instance_id")
        .bind(corpse.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        retained, children,
        "every transform keeps exact child custody"
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("corpse clock")?;
    owner.drain_corpse_deadlines().await;
    let retirements: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(retirements, 0, "final stage retains the source600s window");
    let projected = owner.runtime.lock().await.visible_corpses();
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].item.item_instance_id, corpse);
    assert_eq!(projected[0].item.production_key, "oteryn:item.tibia.i52116");
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-601000 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    restart_lifecycle(h, "retire")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("retirement clock")?;
    owner.drain_corpse_deadlines().await;
    assert!(owner.runtime.lock().await.visible_corpses().is_empty());
    let retired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(retired, i64::try_from(children.len())? + 1);
    let stages: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_field_temporal_receipts r JOIN game_spell_field_temporal_audit a USING(transaction_id) WHERE r.item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        stages, 3,
        "exactly three durable transforms and audit receipts"
    );
    Ok(())
}

// The process receives only a database name and recovery-store locator. It
// reconstructs its own root, seal, node proof and lane; no parent timer survives.
#[test]
fn newhaven_corruptor_lifecycle_process_restart() -> TestResult {
    let Ok(database) = std::env::var("NEWHAVEN_RESTART_DATABASE") else {
        return Ok(());
    };
    let parent = std::env::var("NEWHAVEN_RESTART_PARENT")?;
    let mode = std::env::var("NEWHAVEN_RESTART_MODE")?;
    super::kill_reward_e2e::scenario_thread::run("newhaven-corpse-restart", move || {
        run(async |admin| {
            let prefix = admin
                .strip_suffix("/postgres")
                .ok_or("test admin database")?;
            let url = format!("{prefix}/{database}");
            let root = crate::durability::DurabilityRoot::connect_test_runtime(&url)?;
            assert!(root.maintain_ready_once().await?);
            let retained = std::env::temp_dir()
                .join(format!("oteryn-revision-sequencer-parent-{parent}"))
                .join(&database);
            let recovery = crate::character_recovery_fence::CharacterRecoveryStore::open(
                retained,
                "character-primary",
                "game-ops",
            )
            .map_err(debug)?;
            let seal = recovery.seal_current().map_err(debug)?;
            let authority = root.open_character_authority(&seal).await.map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
            let schedule = root
                .read_corpse_decay_schedule(&authority, world, channel)
                .await
                .map_err(debug)?;
            assert_eq!(schedule.corpses.len(), 1);
            assert_eq!(
                schedule.corpses[0].decay_at_unix_ms - schedule.corpses[0].materialized_at_unix_ms,
                1_810_000
            );
            let corpse = schedule.corpses[0].corpse_item_instance_id;
            let node = super::fixture::registered_node(&root, 1).await?;
            let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_lane(
                world, channel,
            )
            .await;
            if mode == "transforms" {
                assert_eq!(
                    root.drain_spell_item_deadlines(
                        &permit,
                        &authority,
                        &node,
                        crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                        1
                    )
                    .await
                    .map_err(debug)?,
                    3
                );
                assert_eq!(
                    root.drain_spell_item_deadlines(
                        &permit,
                        &authority,
                        &node,
                        crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                        1
                    )
                    .await
                    .map_err(debug)?,
                    0
                );
            } else {
                assert_eq!(mode, "retire");
                root.retire_decayed_corpse(
                    &permit,
                    &authority,
                    &node,
                    crate::durability::item_decay_retire::CorpseDecayFence {
                        runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(
                            world, channel,
                        ),
                        scope_ownership_generation:
                            crate::foundation::ScopeOwnershipGeneration::new(1).map_err(debug)?,
                    },
                    corpse,
                )
                .await
                .map_err(debug)?;
                assert!(
                    root.read_corpse_decay_schedule(&authority, world, channel)
                        .await
                        .map_err(debug)?
                        .corpses
                        .is_empty()
                );
            }
            Ok(())
        })
    })
}

fn restart_lifecycle(h: &Harness, mode: &str) -> TestResult {
    let options = h.pool.connect_options();
    let database = options.get_database().ok_or("test database")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "newhaven_corruptor_lifecycle_process_restart",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NEWHAVEN_RESTART_DATABASE", database)
        .env("NEWHAVEN_RESTART_PARENT", std::process::id().to_string())
        .env("NEWHAVEN_RESTART_MODE", mode)
        .output()?;
    assert!(
        output.status.success(),
        "Corruptor restart {mode}: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

async fn wire_command(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_type: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn as wire;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let frame = wire::encode_client_command(
        1,
        &wire::ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        *admitted,
        owner,
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    )
    .await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(next) = end
    else {
        return Err(format!("Newhaven item command connection ended: {end:?}").into());
    };
    *admitted = next;
    let mut cursor = output.as_slice();
    let mut result = None;
    while !cursor.is_empty() {
        let length = usize::try_from(u32::from_be_bytes(
            cursor.get(..4).ok_or("wire length")?.try_into()?,
        ))?;
        let frame = cursor.get(4..4 + length).ok_or("wire frame")?;
        cursor = &cursor[4 + length..];
        let envelope = wire::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() == wire::MessageType::CommandResult {
            let reply = wire::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.status, wire::CommandStatus::Accepted);
            assert!(result.is_none(), "one result for one item command");
            result = Some(reply.payload.to_vec());
        }
    }
    result.ok_or_else(|| "Newhaven item command reply".into())
}
