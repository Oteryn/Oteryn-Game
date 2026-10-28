//! COIN-EXCHANGE-1 through real NPC_TALK dispatch and runtime-role PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn coin_exchange_postgres_dispatch_rates_refusals_replay_and_custody() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "coin exchange PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "coin_exchange", true, false).await?;
        eprintln!("POSTGRES_STARTED COIN-EXCHANGE-1");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let dependencies = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, _activation, _proof) = purchase_fixture::current(id(40));
        let coordinator =
            SpellPremiumCoordinator::new(consumer.clone(), Some(clock), None, Some(h.root.clone()));
        let mut owner = dependencies.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.premium_coordinator = Some(&coordinator);
        owner.admit_premium(id(40), dependencies.admitted.game_session_id);
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        let session = dependencies.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(dependencies.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.paulie".into(),
                            placements: vec![NpcPlacement {
                                floor: -i8::try_from(spot.floor)?,
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };

        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        // Bank and depot contain funds independently of the carried purse.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        // Seed the same relationally complete balance/ledger shape as the bank fixtures.
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(
                238,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1,
            ),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,100000,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)").bind(id(235).as_slice()).bind(id(232).as_slice()).bind(id(234).as_slice()).bind(id(41).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(236).as_slice()).bind(id(237).as_slice()).bind(id(238).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,100000,0,100000,encode($5,'hex')::uuid)").bind(id(233).as_slice()).bind(id(232).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,100000,encode($3,'hex')::uuid)").bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(233).as_slice()).execute(&mut *tx).await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(
                240,
                "oteryn:item.tibia.i3031",
                "definition-r1",
                100,
            ),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,town_id,box_number,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,1)").bind(id(240).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let untouched = outside(&h).await?;
        let mut next = 1;
        let greeted = owner
            .npc_talk(
                dependencies.actor,
                command(next),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "hi".into(),
                },
            )
            .await;
        assert_eq!(greeted.disposition, NpcIntentDisposition::Ok);
        next += 1;
        // Four real dialogues, whole-stack room, merge, insufficient funds, full bag, zero amount.
        for (round, input, count, receiver, fill, words, expected, input_after, output_after) in [
            (
                0,
                "3031",
                100,
                None,
                19,
                vec!["change gold", "1", "yes"],
                "Here you are.",
                0,
                1,
            ),
            (
                1,
                "3035",
                1,
                None,
                0,
                vec!["change platinum", "gold", "1", "yes"],
                "Here you are.",
                0,
                100,
            ),
            (
                2,
                "3035",
                100,
                None,
                0,
                vec!["change platinum", "crystal", "1 crystal", "yes"],
                "Here you are.",
                0,
                1,
            ),
            (
                3,
                "3043",
                1,
                None,
                0,
                vec!["change crystal", "1", "yes"],
                "Here you are.",
                0,
                100,
            ),
            (
                4,
                "3031",
                100,
                Some(("3035", 99)),
                18,
                vec!["change gold", "1", "yes please"],
                "Here you are.",
                0,
                100,
            ),
            (
                5,
                "3031",
                99,
                None,
                0,
                vec!["change gold", "1", "yes"],
                "Sorry, you do not have enough gold coins.",
                99,
                0,
            ),
            (
                6,
                "3035",
                2,
                None,
                19,
                vec!["change platinum", "gold", "1", "yes"],
                "There is not enough room in your backpack.",
                2,
                0,
            ),
            (
                7,
                "3031",
                100,
                None,
                0,
                vec!["change gold", "0"],
                "Sorry, you do not have enough gold coins.",
                100,
                0,
            ),
        ] {
            seed(&h, round, input, count, receiver, fill).await?;
            if round == 7 {
                nest_last_purse(&h).await?;
            }
            let before = inventory(&h).await?;
            let output = match (input, words.get(1).copied()) {
                ("3031", _) | ("3043", _) => "3035",
                ("3035", Some("gold")) => "3031",
                _ => "3043",
            };
            let mut last = 0;
            for text in words {
                // The real owner rate window uses wall time. Keep this test under its limit.
                tokio::time::sleep(std::time::Duration::from_millis(260)).await;
                last = next;
                let outcome = owner
                    .npc_talk(
                        dependencies.actor,
                        command(next),
                        NpcTalkIntent {
                            npc_actor: npc,
                            text: text.into(),
                        },
                    )
                    .await;
                assert!(!outcome.unknown, "round {round} {text}: storage unknown");
                assert_eq!(
                    outcome.disposition,
                    NpcIntentDisposition::Ok,
                    "round {round} {text}"
                );
                let line = outcome.conversation.ok_or("reply")?.lines;
                if text == "yes" || text == "yes please" || text == "0" {
                    assert_eq!(line, [expected], "round {round}");
                }
                next += 1;
            }
            assert_eq!(
                quantity(&h, input).await?,
                input_after,
                "round {round} input"
            );
            assert_eq!(
                quantity(&h, output).await?,
                output_after,
                "round {round} output"
            );
            assert_eq!(outside(&h).await?, untouched, "bank/depot round {round}");
            if round >= 5 {
                assert_eq!(inventory(&h).await?, before, "refusal is atomic");
            }
            if round < 7 {
                let after = inventory(&h).await?;
                let replay = owner
                    .resume_npc_bank(dependencies.actor, command(last))
                    .await
                    .ok_or("retained exchange")?;
                assert!(!replay.unknown);
                assert_eq!(replay.conversation.ok_or("replay reply")?.lines, [expected]);
                assert_eq!(
                    inventory(&h).await?,
                    after,
                    "replay must not exchange twice"
                );
            }
        }

        // A storage failure after acceptance leaves a recoverable original command. Use nested
        // input stacks in this final purse and recover through the connection adapter.
        sqlx::raw_sql("CREATE FUNCTION coinx_test_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected coinx finalization failure'; END $$; CREATE TRIGGER coinx_test_failure BEFORE UPDATE ON game_coin_exchange_receipts FOR EACH ROW EXECUTE FUNCTION coinx_test_failure();").execute(&h.pool).await?;
        let before = inventory(&h).await?;
        for text in ["change gold", "1", "yes"] {
            tokio::time::sleep(std::time::Duration::from_millis(260)).await;
            let outcome = owner
                .npc_talk(
                    dependencies.actor,
                    command(next),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert_eq!(outcome.unknown, text == "yes");
            next += 1;
        }
        assert_eq!(inventory(&h).await?, before);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_coin_exchange_receipts WHERE result=0"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        sqlx::raw_sql("DROP TRIGGER coinx_test_failure ON game_coin_exchange_receipts; DROP FUNCTION coinx_test_failure();").execute(&h.pool).await?;
        let mut stale = fence;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
                .map_err(debug)?;
        let held = owner
            .resume_npc_bank(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: next - 1,
                    item_fence: Some(stale),
                },
            )
            .await
            .ok_or("held stale exchange")?;
        assert!(held.unknown);
        assert_eq!(inventory(&h).await?, before);
        let recovered = owner
            .resume_npc_bank(dependencies.actor, command(next - 1))
            .await
            .ok_or("pending original exchange")?;
        assert!(!recovered.unknown);
        assert_eq!(
            recovered.conversation.ok_or("recovered reply")?.lines,
            ["Here you are."]
        );
        assert_eq!(quantity(&h, "3031").await?, 0);
        assert_eq!(quantity(&h, "3035").await?, 1);
        assert_eq!(outside(&h).await?, untouched);
        // Retirement cannot lose the receipt, nor may a replay reopen a retired conversation.
        owner.states.npc_conversations.forget(session);
        owner.states.chat.forget(session);
        let after = inventory(&h).await?;
        let replay = owner
            .resume_npc_bank(dependencies.actor, command(next - 1))
            .await
            .ok_or("historical exchange")?;
        assert!(!replay.unknown);
        assert_eq!(replay.conversation.ok_or("historical reply")?.open, None);
        assert_eq!(inventory(&h).await?, after);
        assert!(sqlx::query("UPDATE game_coin_exchange_receipts SET amount=amount+1 WHERE command_id=$1::text::numeric").bind((next-1).to_string()).execute(&h.pool).await.is_err());
        assert_eq!(inventory(&h).await?, after);
        // Reapplying the migration must retain all receipts and proof arms.
        sqlx::raw_sql(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/0317_coin_exchange_receipts.sql"
        )))
        .execute(&h.pool)
        .await?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_coin_exchange_receipts")
                .fetch_one(&h.pool)
                .await?,
            8
        );
        eprintln!("POSTGRES_COMPLETED COIN-EXCHANGE-1 all rates, refusals, replay, bank/depot");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("WITH RECURSIVE t AS (SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid UNION ALL SELECT e.item_instance_id FROM t JOIN (SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries)e ON e.parent_item_instance_id=t.item_instance_id) SELECT coalesce(sum(i.quantity),0)::bigint FROM t JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$2").bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{key}")).fetch_one(&h.pool).await?)
}
async fn inventory(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY i.item_instance_id),'[]') FROM game_item_instances i WHERE i.world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}
async fn outside(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(b)) FROM game_account_bank_balances b),(SELECT jsonb_agg(to_jsonb(e)) FROM game_account_bank_entries e),(SELECT jsonb_agg(to_jsonb(d)) FROM game_item_character_depot_locations d),(SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid))").bind(id(240).as_slice()).fetch_one(&h.pool).await?)
}
async fn seed(
    h: &Harness,
    round: u8,
    input: &str,
    count: u32,
    receiver: Option<(&str, u32)>,
    fill: u8,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    let base = 60 + round * 23;
    let item = |offset, key: &str, count| {
        crate::durability::item_fee_burn::test_support::item(
            base + offset,
            key,
            "definition-r1",
            count,
        )
    };
    sqlx::raw_sql(sqlx::AssertSqlSafe(item(0, "oteryn:item.tibia.i2854", 1)))
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(item(
        1,
        &format!("oteryn:item.tibia.i{input}"),
        count,
    )))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(base).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice()).execute(&mut *tx).await?;
    let mut entries = vec![base + 1];
    if let Some((key, quantity)) = receiver {
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(
            2,
            &format!("oteryn:item.tibia.i{key}"),
            quantity,
        )))
        .execute(&mut *tx)
        .await?;
        entries.push(base + 2);
        if key == input {
            // Domain 9 displays newest ordinals first. The wire helper selects the first
            // matching definition, so keep the intended source ahead of a same-coin sibling.
            entries.swap(0, 1);
        }
    }
    for n in 0..fill {
        let offset = 3 + n;
        sqlx::raw_sql(sqlx::AssertSqlSafe(item(
            offset,
            "oteryn:item.tibia.i3577",
            1,
        )))
        .execute(&mut *tx)
        .await?;
        entries.push(base + offset);
    }
    for (ordinal, entry) in entries.into_iter().enumerate() {
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)").bind(id(41).as_slice()).bind(id(base).as_slice()).bind(id(entry).as_slice()).bind(id(42).as_slice()).bind(i64::try_from(ordinal+1)?).bind(id(246).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn nest_last_purse(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(
            224,
            "oteryn:item.tibia.i2854",
            "definition-r1",
            1,
        ),
    ))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(222).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(221).as_slice()).bind(id(224).as_slice()).bind(id(42).as_slice()).bind(id(246).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)").bind(id(222).as_slice()).bind(id(42).as_slice()).bind(id(224).as_slice()).bind(id(246).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

#[test]
fn coin_use_postgres_wire_atomic_replay_and_refusals() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| coin_use_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "coin use PostgreSQL thread panicked")??;
    Ok(())
}

fn coin_use_scenario() -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget, encode_item_use_intent};
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "coin_use", true, false).await?;
        eprintln!("POSTGRES_STARTED goldconvr");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let channel_pin = ChannelContentPin::from_activation(world, pin.activation_sequence(), pin.identity().server_artifact_digest(), pin.identity().client_artifact_digest(), pin.frame_binding().digest(), pin.qualified_room().map_revision_digest(), (2, 0, 7));
        let deps = Dependencies::with_content(channel_pin, crate::content::qualify_native_entry_room(world).map_err(debug)?, crate::spell::cast::v1_spell_book().map_err(debug)?)?;
        // Item USE and its NotPossible result require the bundle-world capability offer.
        // Supply the real map snapshot required by that offer, rather than a room-only owner.
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let bag = "oteryn:item.tibia.i2854";
        let gold = "oteryn:item.tibia.i3031";
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag definition")?.get() - 1),
            (gold, reference(gold).ok_or("gold definition")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos { x: 2, y: 0, floor: -7 },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical else { return None; };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        ).map_err(debug)?;
        let cells = map.movement_cells(deps.room.movement_cells()).map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let fence = owner.current_quest_fence(deps.admitted.game_session_id).await.map_err(|_| "fence")?.ok_or("fence")?.item_fence();
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(owner.offered_capabilities(), &[4,6,11,15,18]).ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(
            oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
        ));
        assert!(admitted.continuity.selected_capabilities.contains(
            oteryn_protocol_oteryn::world_object::CAPABILITY_WORLD_INTERACTION_V1,
        ));
        let intent = UseIntent { target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?), use_with: None, use_with_map_item: None, use_with_position: None };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        for (command_index, (round, input, count, receiver, fill, expected, remaining, output, output_count)) in [
            (0, "3031", 100, None, 19, UseDisposition::Committed, 0, "3035", 1),
            (1, "3035", 100, None, 0, UseDisposition::Committed, 0, "3043", 1),
            (2, "3035", 2, None, 0, UseDisposition::Committed, 1, "3031", 100),
            (3, "3043", 2, None, 0, UseDisposition::Committed, 1, "3035", 100),
            (4, "3035", 2, None, 19, UseDisposition::NotPossible, 2, "3031", 0),
            (5, "3031", 99, Some(("3031",1)), 0, UseDisposition::NotPossible, 99, "3035", 0),
            (6, "3031", 100, Some(("3035",99)), 18, UseDisposition::Committed, 0, "3035", 100),
            (8, "3035", 2, None, 7, UseDisposition::NotPossible, 2, "3031", 0),
        ].into_iter().enumerate() {
            seed(&h, round, input, count, receiver, fill).await?;
            // A level-one character has free slots but cannot carry the heavy inventory.
            let previous_level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            if round == 8 {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_character_progression_state SET level=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i43872' WHERE item_instance_id IN (SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) AND definition_production_key='oteryn:item.tibia.i3577'").bind(id(244).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            let source = id(61 + round*23);
            let before = h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("before")?;
            let key = format!("oteryn:item.tibia.i{input}");
            let definition = index.definition_ref("Item", &key, "definition-r1").ok_or("coin definition")?;
            let command_id = u64::try_from(command_index)? + 1;
            let (end, result, _) = super::item_craft::exchange(&owner, admitted, command_id, &payload, Some(definition)).await?;
            assert_eq!(result, expected, "round {round}");
            let retained_source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_source) FROM game_coin_exchange_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command_id.to_string()).fetch_one(&h.pool).await?;
            assert_eq!(retained_source, source, "round {round} exact source");
            let after = h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("after")?;
            let source_quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(source_quantity, remaining);
            let output_key = format!("oteryn:item.tibia.i{output}");
            assert_eq!(after.entries.iter().filter(|e| e.item.definition.production_key==output_key).map(|e| e.item.quantity).sum::<u32>(), output_count);
            if expected == UseDisposition::NotPossible { assert_eq!(before, after); }
            // A new composition has no runtime result cache. Its receipt must converge even for
            // retired inputs, and cannot borrow another stack or replay a bank command.
            let mut restarted = deps.owner(&h, &authority);
            restarted.active_generation = Some(active);
            restarted.movement_cells = &cells;
            let replay = restarted.use_item_completion(deps.actor, UseCommand { game_session_id: fence.game_session_id, command_id, item_fence: Some(fence) }, ItemKey::Instance(source)).await.ok_or_else(|| format!("round {round} replay"))?;
            assert_eq!(replay.disposition, expected);
            assert_eq!(h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("replayed")?, after);
            if round == 8 {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_character_progression_state SET level=$2 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(previous_level).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            admitted = end;
        }
        // An accepted command survives a finalization failure. No partial coin mutation is visible;
        // the replacement owner resumes that exact source without relying on the retired view.
        seed(&h, 7, "3031", 100, None, 0).await?;
        let before = h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("failure before")?;
        sqlx::raw_sql("CREATE FUNCTION goldconvr_test_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected conversion failure'; END $$; CREATE CONSTRAINT TRIGGER goldconvr_test_failure AFTER UPDATE ON game_coin_exchange_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION goldconvr_test_failure();").execute(&h.pool).await?;
        let command = UseCommand { game_session_id: fence.game_session_id, command_id: 9, item_fence: Some(fence) };
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert!(owner.use_item_completion(deps.actor, UseCommand { item_fence: Some(stale), ..command }, ItemKey::Instance(id(222))).await.is_none());
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_coin_exchange_receipts WHERE command_id=9").fetch_one(&h.pool).await?, 0);
        assert_eq!(h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("stale unchanged")?, before);
        assert!(owner.use_item_completion(deps.actor, command, ItemKey::Instance(id(222))).await.is_none());
        assert_eq!(h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("failure after")?, before);
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_coin_exchange_receipts WHERE result=0 AND command_id=9").fetch_one(&h.pool).await?;
        assert_eq!(pending, 1);
        sqlx::raw_sql("DROP TRIGGER goldconvr_test_failure ON game_coin_exchange_receipts; DROP FUNCTION goldconvr_test_failure();").execute(&h.pool).await?;
        let mut restarted = deps.owner(&h, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = &cells;
        assert!(restarted.resume_npc_bank(deps.actor, command).await.is_none());
        assert_eq!(h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("bank cannot resume use")?, before);
        // A different source under the same command is rejected before executing the pending use.
        assert!(restarted.use_item_completion(deps.actor, command, ItemKey::Instance(id(221))).await.is_none());
        assert_eq!(h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("mismatch")?, before);
        let recovered = restarted.use_item_completion(deps.actor, command, ItemKey::Instance(id(222))).await.ok_or("pending recovery")?;
        assert_eq!(recovered.disposition, UseDisposition::Committed);
        let after = h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("recovered")?;
        assert_eq!(after.entries.len(), 1);
        assert_eq!(after.entries[0].item.definition.production_key, "oteryn:item.tibia.i3035");
        assert_eq!(after.entries[0].item.quantity, 1);
        assert_eq!(restarted.use_item_completion(deps.actor, command, ItemKey::Instance(id(222))).await.ok_or("recovered replay")?.disposition, UseDisposition::Committed);
        assert_eq!(h.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("recovered unchanged")?, after);
        assert!(restarted.resume_npc_bank(deps.actor, command).await.is_none());
        assert!(sqlx::query("UPDATE game_coin_exchange_receipts SET item_source=encode($1,'hex')::uuid WHERE command_id=9").bind(id(221).as_slice()).execute(&h.pool).await.is_err());
        drop(restarted);
        sqlx::raw_sql(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../apps/game-server/migrations/0336_item_coin_conversion.sql"))).execute(&h.pool).await?;
        eprintln!("POSTGRES_COMPLETED goldconvr");
        drop(owner); drop(authority); drop(seal);
        h.cleanup().await
    })
}
