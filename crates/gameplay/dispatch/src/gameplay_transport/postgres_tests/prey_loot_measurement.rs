//! PREY-DIFF: the real auto-attack/kill/corpse owner supplies measured loot to history.
use super::*;
use super::prey_difficulty::{HistoryCounters, assert_history, selected_at};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, TakeOutcome};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::prey as wire;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::CommandStatus;

const RAT: &str = "oteryn:creature.rat";
const PLAYER_CELL: crate::foundation::MovementLocalPosition = crate::foundation::MovementLocalPosition {
    x: 2, y: 11, floor: -7,
};
const RAT_CELL: crate::foundation::MovementLocalPosition = crate::foundation::MovementLocalPosition {
    x: 1, y: 11, floor: -7,
};

#[test]
fn prey_diff_real_kill_values_committed_quantities_and_replay_keeps_one_history_measurement() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED route_less");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "prey committed loot case panicked")??;
    eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED");
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "prey_committed_loot", false, false).await?;
        prepare_current_session(&h).await?;
        super::daily_reward::epoch(&h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        // The existing source-world fixture binds native build training to the
        // Character root's content revision; the real vocation writer checks it.
        let (generation, pin) =
            super::death_ml_loss_composed::production_generation_with_source(
                world,
                "content-1",
                Some(include_bytes!("../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json")),
            )?;
        let active = generation.active().ok_or("activated source-world generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let catalogue = crate::durability::prey::PreyCatalogue::try_from(native).map_err(debug)?;
        let room = pin.qualified_room().clone();
        let channel_pin = ChannelContentPin::from_activation(
            world, pin.activation_sequence(), pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(), pin.frame_binding().digest(),
            room.map_revision_digest(), (PLAYER_CELL.x, PLAYER_CELL.y, PLAYER_CELL.floor),
        );
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),NULL,$4,$5,$6)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::with_content(
            channel_pin.clone(), room.clone(), crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let rat = {
            let mut runtime = deps.runtime.lock().await;
            native.install_companion_policies(&mut runtime).map_err(debug)?;
            runtime.bind_attacker_lease(actor, session, crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2,
            ).map_err(debug)?).map_err(debug)?;
            runtime.initialize_first_entry_position(actor).map_err(debug)?;
            // Select a real positive death occurrence, as the existing native creature
            // settlement fixture does. No drop chance, table or item quantity is changed.
            let reward = native.reward_table().row(RAT).map_err(debug)?;
            let average_micros = catalogue.rules().creatures.get(RAT).ok_or("rat valuation")?.loot_per_kill_micros;
            let mut positive_generation = None;
            for candidate in 1..=20_000 {
                let death = oteryn_gameplay_items::loot_plan::LootPlanDeathKey::new(id(42), id(43), 1, 2, candidate);
                let plan = oteryn_gameplay_items::loot_plan::plan_creature_loot(
                    death, &reward.loot_table_ref, &reward.loot_table,
                ).map_err(debug)?;
                let planned_gold: u128 = plan.entries.iter().map(|entry|
                    u128::from(entry.quantity) * u128::from(catalogue.rules().npc_sell_prices
                        .get(&entry.item.production_key).copied().unwrap_or(0))).sum();
                if planned_gold * 1_000_000 > u128::from(average_micros)
                    && plan.entries.iter().any(|entry| entry.quantity > 1
                        && catalogue.rules().npc_sell_prices.get(&entry.item.production_key).is_some_and(|price| *price > 0)) {
                    positive_generation = Some(candidate);
                    break;
                }
            }
            for _ in 1..positive_generation.ok_or("bounded positive native quantity occurrence")? {
                let unused = runtime.reserve_fresh_session(GameSessionId::decode(&id(245)).map_err(debug)?)
                    .map_err(debug)?;
                runtime.rollback_definitely_uncommitted(unused).map_err(debug)?;
            }
            runtime.realize_native_qualification_spawn(channel_pin, RAT, RAT_CELL).map_err(debug)?;
            let creature = runtime.visible_entities().creatures.first().copied().ok_or("native rat")?;
            EntityRef { identity: creature.actor.placement_identity(), generation: creature.generation }
        };
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.clock_origin = std::time::Instant::now().checked_sub(Duration::from_secs(60)).ok_or("owner clock")?;
        {
            let runtime = owner.runtime.lock().await;
            assert!(owner.states.spell_states.lock().await.initialize(
                &runtime, actor, session,
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Monk, level: 50, max_health: 185,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                }, (3, 7_000_000), owner.owner_now(),
            ).is_some());
        }
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(session).is_some());
        let current = crate::gameplay_transport::character_gameplay_fence::current(&owner, session, false)
            .await.ok_or("current character fence")?;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let offered = crate::gameplay_transport::prey::offered(
            crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES, true,
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(&offered, &[1, 6, 140])
            .ok_or("Prey capabilities")?;
        let formula = native.training_formula().ok_or("training formula")?;
        let choice = crate::gameplay_transport::vocation_choice::vocation_choice_request(
            crate::durability::character_build::BuildOccurrence::from_bytes(id(94)).map_err(debug)?,
            crate::durability::character_build::DurableBuildState::default(), "monk", formula, None,
        ).map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(&owner, session, false)
            .await.ok_or("vocation fence")?;
        owner.revision_sequencer.acquire(current.character_id).await.commit_build(
            owner.root, owner.character, owner.holder, current, choice, formula, None,
        ).await.map_err(|error| format!("prey loot vocation commit: {error:?}"))?;
        assert!(owner.admit_prey(admitted).await);
        let available = send(&owner, &mut admitted, &wire::Request::Query).await?;
        let (index, acquisition_free_rerolls) = if let Some(index) = available.slots[0].creatures.iter().position(|key| key == RAT) {
            (u8::try_from(index + 1)?, 0)
        } else {
            // Admission after vocation choice creates the real first draw and receipt.
            // Use that retained occurrence for the isolated-DB prior-offer fixture;
            // a vocationless no-op cannot supply this required FK. Only the rat is
            // still unseen, and the real free-reroll command must draw it.
            let draw_occurrence: Vec<u8> = sqlx::query_scalar(
                "SELECT uuid_send(occurrence) FROM game_character_prey_receipts WHERE character_id=encode($1,'hex')::uuid ORDER BY committed_character_revision LIMIT 1",
            ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(draw_occurrence.len(), 16);
            let prior: Vec<_> = catalogue.rules().creatures.keys().filter(|key| key.as_str() != RAT).cloned().collect();
            sqlx::query("INSERT INTO game_character_prey_offered(character_id,creature_key,reset_epoch,draw_occurrence) SELECT encode($1,'hex')::uuid,k,1,encode($3,'hex')::uuid FROM unnest($2::text[]) k ON CONFLICT(character_id,creature_key) DO NOTHING")
                .bind(id(41).as_slice()).bind(prior).bind(draw_occurrence).execute(&h.pool).await?;
            let rerolled = send(&owner, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
            assert_eq!(rerolled.slots[0].creatures, [RAT]);
            (1, 1)
        };
        let selected = send(&owner, &mut admitted, &wire::Request::Select { slot: 1, index }).await?;
        assert_eq!(selected.slots[0].selected, RAT);
        let started = selected_at(&h, 1).await?;
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(owner.states.attack.lock().await.set_target(
                &runtime, Some(&room), actor, session, owner.owner_now(), Some(rat), 1,
            ), AttackIntentDisposition::Ok);
        }
        let mut swings = 0;
        loop {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner.states.attack.lock().await.kills().has_session(session) { break; }
            swings += 1;
            assert!(swings < 64, "native rat dies to actual auto-attack swings");
            owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_secs(3)).ok_or("advance attack clock")?;
        }
        let (facts, target, observed) = {
            let attack = owner.states.attack.lock().await;
            let TakeOutcome::Taken(taken) = attack.kills().take(session, 2) else {
                return Err("actual queued rat kill".into());
            };
            let entry = taken.entry().ok_or("queued death facts")?;
            let checkpoint = entry.prey_checkpoint.as_ref().ok_or("kill freezes selected Prey")?;
            assert_eq!(checkpoint.creature, RAT);
            assert_eq!(checkpoint.selection_epoch, selected.slots[0].selection_epoch);
            assert_eq!(checkpoint.kills, 1);
            let replay = (entry.facts, entry.target, entry.activity_observed_at);
            taken.requeue();
            replay
        };
        drain(&owner, session).await?;
        let actual: Vec<(String, i64)> = sqlx::query_as(
            "SELECT i.definition_production_key,i.quantity FROM game_item_corpse_container_entries e JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id JOIN game_item_mint_receipts r ON r.item_instance_id=i.item_instance_id ORDER BY r.draw_ordinal",
        ).fetch_all(&h.pool).await?;
        assert!(actual.iter().any(|(key, quantity)| *quantity > 1
            && catalogue.rules().npc_sell_prices.get(key).is_some_and(|price| *price > 0)),
            "the actual committed corpse contains a valued multi-item quantity");
        // The expected value comes only from the actual Item rows and generated NPC/coin
        // prices. Neither the average loot model nor the uncommitted plan is the oracle.
        let actual_gold = actual.iter().try_fold(0_u64, |total, (key, quantity)| -> TestResult<u64> {
            let value = catalogue.rules().npc_sell_prices.get(key).copied().unwrap_or(0)
                .checked_mul(u64::try_from(*quantity)?).ok_or("quantity value overflow")?;
            Ok(total.checked_add(value).ok_or("corpse value overflow")?)
        })?;
        assert!(actual_gold > 0);
        assert!(u128::from(actual_gold) * 1_000_000 > u128::from(
            catalogue.rules().creatures.get(RAT).ok_or("rat valuation")?.loot_per_kill_micros,
        ), "this actual corpse differs from the expected-loot table");
        let measured = send(&owner, &mut admitted, &wire::Request::Query).await?;
        let consumed = 7200 - i64::from(measured.slots[0].hunting_seconds);
        assert_measurement(&h, actual_gold).await?;
        let mint_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts")
            .fetch_one(&h.pool).await?;
        let prey_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_prey_receipts")
            .fetch_one(&h.pool).await?;
        {
            let mut runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let attack = owner.states.attack.lock().await;
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink { prey: Some(&states), activity: &[], queue: attack.kills(), rewards: native.reward_table(), charms: None },
                target, facts.principal, facts.death_at_ms, observed,
            );
            assert!(attack.kills().has_session(session), "same death replay reaches the real kill queue");
        }
        drain(&owner, session).await?;
        assert_measurement(&h, actual_gold).await?;
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_mint_receipts").fetch_one(&h.pool).await?, mint_count);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_receipts").fetch_one(&h.pool).await?, prey_count);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_xp_receipts").fetch_one(&h.pool).await?, 1);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key=$1 AND materialized_at IS NOT NULL")
            .bind(crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY).fetch_one(&h.pool).await?, 1);
        // Advance the actual World reset fixture so cancellation has a fresh list pool.
        // If acquisition used the free reroll, fund a real paid cancellation only after
        // all corpse-value and replay assertions, using the existing native coin fixture.
        if acquisition_free_rerolls != 0 { seed_cancellation_coins(&h, native).await?; }
        super::daily_reward::epoch(&h, 2).await?;
        let cancel_id = admitted.continuity.next_command_id;
        let cancel = wire::Request::ListReroll { slot: 1 };
        let ended = send(&owner, &mut admitted, &cancel).await?;
        assert_eq!(ended.slots[0].state, 1);
        assert_history(&h, &selected.slots[0], &catalogue, started, "CANCELLED", HistoryCounters {
            consumed, wildcards: 0, free_rerolls: acquisition_free_rerolls, gold_rerolls: 0,
            kills: 1, loot_gold: i64::try_from(actual_gold)?, pick_gold: 0,
        }).await?;
        let bytes = wire::encode_request(&cancel).map_err(debug)?;
        let crate::gameplay_transport::guild_intent::GuildReply::Payload(replayed) = owner.prey_command(admitted, cancel_id, &bytes).await else {
            return Err("end receipt replay".into());
        };
        assert_eq!(wire::decode_reply(&replayed).map_err(debug)?, ended);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_history").fetch_one(&h.pool).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn send(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, request: &wire::Request) -> TestResult<wire::Reply> {
    let bytes = wire::encode_request(request).map_err(debug)?;
    let id = admitted.continuity.next_command_id;
    let values = super::prey::exchange(owner, admitted, &[(id, &bytes)]).await?;
    assert_eq!(values[0].0, CommandStatus::Accepted);
    let reply = wire::decode_reply(&values[0].1).map_err(debug)?;
    assert_eq!(reply.status, 0);
    Ok(reply)
}

async fn drain(owner: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) -> TestResult {
    let mut passes = 0;
    while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
        passes += 1;
        assert!(passes < 8, "the actual kill and its Prey measurement settle");
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Ok(())
}

async fn assert_measurement(h: &Harness, gold: u64) -> TestResult {
    let (kills, value): (i64, i64) = sqlx::query_as(
        "SELECT chosen_kills,chosen_loot_gold FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=1",
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(kills, 1);
    assert_eq!(value, i64::try_from(gold)?);
    Ok(())
}

// Same isolated native item fixture as prey_difficulty::seed_coins. This supplies
// cancellation payment after the measurement oracle; it never supplies loot or counters.
async fn seed_cancellation_coins(
    h: &Harness,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
) -> TestResult {
    let index = native.item_index().ok_or("items")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    for (seed, key, quantity) in [
        (210, "oteryn:item.tibia.i2854", 1),
        (212, "oteryn:item.tibia.i3043", 100),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(
                seed, key, index.revision(key).ok_or("revision")?, quantity,
            ),
        )).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(42).as_slice()).bind(id(214).as_slice())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(212).as_slice()).bind(id(42).as_slice()).bind(id(215).as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
