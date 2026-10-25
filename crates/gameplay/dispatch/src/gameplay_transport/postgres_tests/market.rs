//! MARKET-1: installed migration and real depot-admitted command route.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
use crate::map::facts::ItemDefinition;
use oteryn_protocol_oteryn::market::*;

#[test]
fn market_postgres_settlement_replay_and_conservation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "market scenario panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "market_settlement", true, false).await?;
        super::reset();
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        assert_eq!(harness.root.active_world_reset_epoch(world_id).await?, None);

        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let backpack_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", "oteryn:item.tibia.i2854", "definition-r1")
                .ok_or("backpack ref")?,
        )
        .ok_or("backpack definition")?;
        let coin_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1")
                .ok_or("coin ref")?,
        )
        .ok_or("coin definition")?;

        let box_id = index
            .definition_ref(
                "Item",
                &backpack_definition.production_key,
                &backpack_definition.revision_ref,
            )
            .ok_or("box id")?
            .get()
            - 1;
        let coin_id = index
            .definition_ref(
                "Item",
                &coin_definition.production_key,
                &coin_definition.revision_ref,
            )
            .ok_or("coin id")?
            .get()
            - 1;
        let (bytes, mut bundle_pins) = bundle_fixture::bundle_with_item_ids(
            (&backpack_definition.production_key, box_id),
            (&coin_definition.production_key, coin_id),
        );
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        let locker = bundle
            .sectors
            .iter_mut()
            .flat_map(|s| &mut s.tiles)
            .find(|t| t.x == 6)
            .ok_or("locker tile")?;
        locker.items.get_mut(1).ok_or("locker item")?.attrs.depot = Some(0);
        let bytes =
            oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
                .map_err(debug)?;
        bundle_pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest;
        let base = crate::map::load(&bytes, &bundle_pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: bundle_pins,
                start: crate::map::overlay::TilePos {
                    x: 7,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(ItemDefinition {
                    reference: index.definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key == backpack_definition.production_key),
                    blocks_projectile: false,
                    pickupable: key == coin_definition.production_key,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (7, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        let session = deps.admitted.game_session_id;
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        harness
            .root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(harness.root.clone()),
            );
        owner.premium_coordinator = Some(&coordinator);
        deps.initialize(&owner).await;
        owner.admit_premium(id(40), session);
        owner.admit_promotion(session).await;
        assert!(owner.session_premium_gate(session).await.premium_current());
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(session)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        // Retained native custody/balance prefixes are fixture setup only. Every
        // mutation under test below executes with all production guards enabled.
        let index = native.item_index().ok_or("item index")?;
        let mut ware = None;
        for reference in 1..=100_000 {
            let Some((key, revision)) =
                index.definition_at(std::num::NonZeroU32::new(reference).ok_or("reference")?)
            else {
                break;
            };
            let Some(policy) = native.item_policy(key, revision) else {
                continue;
            };
            if matches!(&policy.record().semantics.trade_restrictions,crate::content::ReferenceItemField::Known(trade) if trade.marketable==crate::content::ReferenceItemField::Known(true))
                && policy.record().admission.stack_class
                    == crate::content::native_gameplay::NativeItemStackClass::StackCapable
            {
                ware = Some((key.to_owned(), revision.to_owned(), reference));
                break;
            }
        }
        let (key, revision, reference) = ware.ok_or("marketable native ware")?;
        let buyer_character = crate::durability::guild_chat_fixture::sid(90, 1);
        let buyer_account = crate::durability::guild_chat_fixture::sid(90, 2);
        crate::durability::guild_chat_fixture::historical_character(
            &harness.pool,
            90,
            crate::domain::CharacterId::from_bytes(buyer_character).map_err(debug)?,
            buyer_account,
            id(42),
            "Buyer",
        )
        .await?;
        let buyer_fence = crate::durability::guild_chat_fixture::historical_session(
            &harness.pool,
            90,
            crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: 1,
                ..fence
            },
        )
        .await?;

        let mut seed = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        for (account, character, tag) in [
            (id(40), id(41), 210_u8),
            (buyer_account, buyer_character, 220),
        ] {
            sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,1,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)")
                .bind(id(tag).as_slice()).bind(id(tag+1).as_slice()).bind(id(tag+2).as_slice()).bind(character.as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(tag+3).as_slice()).bind(id(tag+4).as_slice()).bind(id(tag+6).as_slice()).execute(&mut *seed).await?;
            sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,1,9999,10000,encode($5,'hex')::uuid)")
                .bind(id(tag+5).as_slice()).bind(id(tag+1).as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(character.as_slice()).execute(&mut *seed).await?;
            sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,10000,encode($3,'hex')::uuid)")
                .bind(account.as_slice()).bind(id(42).as_slice()).bind(id(tag+5).as_slice()).execute(&mut *seed).await?;
        }
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,10,1,encode($5,'hex')::uuid)")
            .bind(id(200).as_slice()).bind(id(42).as_slice()).bind(&key).bind(&revision).bind(id(201).as_slice()).execute(&mut *seed).await?;
        sqlx::query(
            "INSERT INTO game_character_depot_box_counters VALUES(encode($1,'hex')::uuid,0,1,2,1)",
        )
        .bind(id(41).as_slice())
        .execute(&mut *seed)
        .await?;
        sqlx::query("INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,town_id,box_number,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,0,1,1)")
            .bind(id(200).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *seed).await?;
        seed.commit().await?;

        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 18, 37, 63],
            )
            .ok_or("selected Market capability")?;
        assert!(admitted.continuity.selected_capabilities.contains(63));
        let sell = MarketRequest {
            action: 6,
            definition: reference,
            offer: [0; 16],
            amount: 10,
            price: 100,
            anonymous: true,
            page: 0,
        };
        let empty = MarketRequest {
            action: 1,
            definition: 0,
            offer: [0; 16],
            amount: 0,
            price: 0,
            anonymous: false,
            page: 0,
        };
        harness
            .root
            .advance_market(&authority, &harness.node, deps.admitted.world_id)
            .await
            .map_err(|e| format!("Market World step before wire: {e:?}"))?;
        let mut input = Vec::new();
        for (offset, request) in [
            sell,
            empty,
            MarketRequest {
                action: 2,
                definition: reference,
                ..empty
            },
            MarketRequest { action: 3, ..empty },
            MarketRequest { action: 4, ..empty },
        ]
        .into_iter()
        .enumerate()
        {
            let payload = encode_request(request).map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: u64::try_from(offset)? + 1,
                    command_type: 80,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            input.extend(u32::try_from(frame.len())?.to_be_bytes());
            input.extend(frame);
        }

        let mut unsupported = admitted;
        unsupported.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 18, 37],
            )
            .ok_or("selected depot without Market")?;
        let before_unsupported = snapshot(&harness).await?;
        // Observe the unselected client's snapshot without consuming this
        // fixture session's command ingress before the selected-client scenario.
        let (_, unsupported_output) =
            super::weather_wire::fixture_access::serve_input(&owner, unsupported, &[]).await?;
        assert!(!unsupported_output.is_empty());
        let rejected = owner
            .market_command(unsupported, 1, &encode_request(sell).map_err(debug)?)
            .await
            .ok_or("unselected Market refusal")?;
        assert_eq!(rejected.get(1), Some(&9));
        let mut cursor = unsupported_output.as_slice();
        while !cursor.is_empty() {
            let length =
                u32::from_be_bytes(cursor.get(..4).ok_or("frame length")?.try_into()?) as usize;
            let frame = cursor.get(4..4 + length).ok_or("frame")?;
            cursor = &cursor[4 + length..];
            let env = oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
            match env.message_type() {
                oteryn_protocol_oteryn::MessageType::SnapshotChunk => {
                    let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(env.payload())
                        .map_err(debug)?;
                    assert!(
                        domains
                            .iter()
                            .all(|domain| domain.domain_id != STATE_DOMAIN_MARKET)
                    );
                }
                oteryn_protocol_oteryn::MessageType::StateDelta => {
                    let delta =
                        oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                    assert_ne!(delta.domain_id, STATE_DOMAIN_MARKET);
                }
                oteryn_protocol_oteryn::MessageType::CommandResult => {
                    let result = oteryn_protocol_oteryn::decode_command_result(env.payload())
                        .map_err(debug)?;
                    assert_ne!(
                        result.status,
                        oteryn_protocol_oteryn::CommandStatus::Accepted
                    );
                }
                _ => {}
            }
        }
        assert_eq!(before_unsupported, snapshot(&harness).await?);

        let (end, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        let mut replies = Vec::new();
        let mut deltas = Vec::new();
        let mut cursor = output.as_slice();
        while !cursor.is_empty() {
            let length =
                u32::from_be_bytes(cursor.get(..4).ok_or("frame length")?.try_into()?) as usize;
            let frame = cursor.get(4..4 + length).ok_or("frame")?;
            cursor = &cursor[4 + length..];
            let env = oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
            if env.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                let result =
                    oteryn_protocol_oteryn::decode_command_result(env.payload()).map_err(debug)?;
                replies.push(result.payload.to_vec());
            } else if env.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
                let delta =
                    oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                if delta.domain_id == STATE_DOMAIN_MARKET {
                    deltas.push(delta.payload.to_vec());
                }
            }
        }
        assert_eq!(
            deltas, replies,
            "Market result fields survive the domain relay"
        );
        if replies.is_empty() {
            let diagnostic = harness
                .root
                .commit_market(
                    &authority,
                    &harness.node,
                    fence,
                    CommandRef::new(session, CommandId::new(1).map_err(debug)?),
                    sell,
                    Some(crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key.clone(),
                        revision_ref: revision.clone(),
                    }),
                    0,
                )
                .await;
            return Err(format!("market wire stopped; writer diagnostic: {diagnostic:?}").into());
        }
        assert_eq!(replies.len(), 5, "{end}");
        assert!(
            replies.iter().all(|reply| reply.get(1) == Some(&1)),
            "{replies:?}"
        );
        let reply = &replies[0];
        assert_eq!(reply.get(1), Some(&1), "sell reply {reply:?}");
        let offer_text: String =
            sqlx::query_scalar("SELECT offer_id::text FROM game_market_offers LIMIT 1")
                .fetch_one(&harness.pool)
                .await?;
        let offer = uuid(&offer_text)?;
        let accept = MarketRequest {
            action: 8,
            definition: 0,
            offer,
            amount: 4,
            price: 0,
            anonymous: false,
            page: 0,
        };
        let command = CommandRef::new(
            buyer_fence.game_session_id,
            CommandId::new(1).map_err(debug)?,
        );
        let first = harness
            .root
            .commit_market(
                &authority,
                &harness.node,
                buyer_fence,
                command,
                accept,
                Some(crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.clone(),
                    revision_ref: revision.clone(),
                }),
                0,
            )
            .await?;
        assert_eq!(first.code, 1);
        let committed_snapshot = snapshot(&harness).await?;
        // Unknown acknowledgement: discard the first result and retry the exact request.
        let replay = harness
            .root
            .commit_market(
                &authority,
                &harness.node,
                buyer_fence,
                command,
                accept,
                Some(crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.clone(),
                    revision_ref: revision.clone(),
                }),
                0,
            )
            .await?;
        assert_eq!(first, replay);
        let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
            connection_generation: crate::foundation::ConnectionGeneration::new(2)
                .map_err(debug)?,
            ..buyer_fence
        };
        assert!(
            harness
                .root
                .commit_market(
                    &authority,
                    &harness.node,
                    stale,
                    command,
                    accept,
                    Some(crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key.clone(),
                        revision_ref: revision.clone()
                    }),
                    0
                )
                .await
                .is_err()
        );
        for table in [
            "game_market_offers",
            "game_market_books",
            "game_market_settlements",
            "game_market_item_lines",
            "game_market_value_lines",
            "game_market_operations",
        ] {
            let can_write: bool = sqlx::query_scalar(
                "SELECT has_table_privilege('oteryn_game_runtime',$1,'INSERT,UPDATE,DELETE')",
            )
            .bind(table)
            .fetch_one(&harness.pool)
            .await?;
            assert!(!can_write, "runtime has a broad write grant on {table}");
        }

        assert_eq!(committed_snapshot, snapshot(&harness).await?);
        // Counterfactual in this disposable database only: returning a freshly
        // computed refusal instead of the immutable outcome must break replay equality.
        let original:String=sqlx::query_scalar("SELECT pg_get_functiondef(p.oid) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname=current_schema() AND p.proname='game_market_write'").fetch_one(&harness.pool).await?;
        let mutant = original.replace(
            "RETURN previous.outcome;",
            "RETURN decode('06','hex')||uuid_send(o);",
        );
        assert_ne!(mutant, original);
        sqlx::raw_sql(sqlx::AssertSqlSafe(mutant))
            .execute(&harness.pool)
            .await?;
        let counterfactual = harness
            .root
            .commit_market(
                &authority,
                &harness.node,
                buyer_fence,
                command,
                accept,
                Some(crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.clone(),
                    revision_ref: revision.clone(),
                }),
                0,
            )
            .await;
        sqlx::raw_sql(sqlx::AssertSqlSafe(original))
            .execute(&harness.pool)
            .await?;
        assert_ne!(first, counterfactual?);
        assert_eq!(committed_snapshot, snapshot(&harness).await?);

        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT sum(i.quantity)::bigint FROM game_item_character_inbox_locations l JOIN game_item_instances i USING(item_instance_id)").fetch_one(&harness.pool).await?,4);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT sum(balance)::bigint FROM game_account_bank_balances"
            )
            .fetch_one(&harness.pool)
            .await?,
            19980
        );
        let cancel = MarketRequest {
            action: 7,
            definition: 0,
            offer,
            amount: 0,
            price: 0,
            anonymous: false,
            page: 0,
        };
        let cancel = harness
            .root
            .commit_market(
                &authority,
                &harness.node,
                fence,
                CommandRef::new(session, CommandId::new(6).map_err(debug)?),
                cancel,
                None,
                0,
            )
            .await?;
        assert_eq!(cancel.code, 1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT sum(i.quantity)::bigint FROM game_item_character_inbox_locations l JOIN game_item_instances i USING(item_instance_id)").fetch_one(&harness.pool).await?,10);
        let before = snapshot(&harness).await?;
        for (number, request, expected) in [
            (
                7,
                MarketRequest {
                    action: 5,
                    amount: 1,
                    price: 100_000,
                    ..sell
                },
                2,
            ),
            (8, sell, 3),
        ] {
            let refused = harness
                .root
                .commit_market(
                    &authority,
                    &harness.node,
                    fence,
                    CommandRef::new(session, CommandId::new(number).map_err(debug)?),
                    request,
                    Some(crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key.clone(),
                        revision_ref: revision.clone(),
                    }),
                    0,
                )
                .await?;
            assert_eq!(refused.code, expected);
            assert_eq!(before, snapshot(&harness).await?);
        }
        let history = harness
            .root
            .query_market(
                &authority,
                &harness.node,
                fence,
                MarketRequest {
                    action: 4,
                    definition: 0,
                    offer: [0; 16],
                    amount: 0,
                    price: 0,
                    anonymous: false,
                    page: 0,
                },
                String::new(),
            )
            .await?;
        assert_eq!(history.len(), 2);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
fn uuid(text: &str) -> TestResult<[u8; 16]> {
    let hex = text.replace('-', "");
    let mut out = [0; 16];
    for (n, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[n * 2..n * 2 + 2], 16)?;
    }
    Ok(out)
}
async fn snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(b) ORDER BY account_id) FROM game_account_bank_balances b),(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),(SELECT jsonb_agg(to_jsonb(o) ORDER BY offer_id) FROM game_market_offers o),(SELECT jsonb_agg(to_jsonb(l) ORDER BY item_instance_id) FROM game_item_character_inbox_locations l),(SELECT jsonb_agg(to_jsonb(l) ORDER BY item_instance_id) FROM game_item_market_escrow_locations l))::text").fetch_one(&h.pool).await?)
}

#[test]
fn market_postgres_migration_diagnostic() -> TestResult {
    run(async |admin| {
        use sqlx::Connection;
        let mut connection = sqlx::PgConnection::connect(&admin)
            .await
            .map_err(|_| "PG connect")?;
        let name = format!(
            "market_diag_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut connection)
            .await?;
        let options: sqlx::postgres::PgConnectOptions = admin.parse().map_err(|_| "PG options")?;
        let mut database = sqlx::PgConnection::connect_with(&options.database(&name))
            .await
            .map_err(|_| "PG database connect")?;
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../apps/game-server/migrations");
        let migrator = sqlx::migrate::Migrator::new(directory.as_path()).await?;
        let result = migrator
            .run(&mut database)
            .await
            .map_err(|e| format!("{e:?}").into());
        database.close().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {name} WITH (FORCE)"
        )))
        .execute(&mut connection)
        .await?;
        result
    })
}

#[test]
fn market_postgres_expiry_recovery_returns_buy_escrow_once() -> TestResult {
    expiry_recovery(false)
}

#[test]
fn market_postgres_expiry_clears_pending_match() -> TestResult {
    expiry_recovery(true)
}

fn expiry_recovery(pending_match: bool) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "market_expiry", true, false).await?;

        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Retained prefix: an already paid-for buy offer, expired before restart.
        sqlx::query("INSERT INTO game_market_books VALUES(encode($1,'hex')::uuid,'oteryn:item.tibia.i3031')").bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,created_at_us,expires_at_us,state,escrow_gold,held_credit_gold) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'oteryn:item.tibia.i3031','definition-r1',1,encode($3,'hex')::uuid,encode($4,'hex')::uuid,100,1,1,1,2592000000001,1,100,0)")
            .bind(id(190).as_slice()).bind(id(42).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
        if pending_match {
            sqlx::query("UPDATE game_market_offers SET matching_pending=true WHERE offer_id=encode($1,'hex')::uuid")
                .bind(id(190).as_slice()).execute(&h.pool).await?;
        }
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;

        h.root
            .advance_market(&authority, &h.node, world)
            .await
            .map_err(|e| format!("Market expiry step: {e:?}"))?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                .fetch_one(&h.pool)
                .await?,
            100
        );
        assert_eq!(
            sqlx::query_scalar::<_, i16>("SELECT state FROM game_market_offers")
                .fetch_one(&h.pool)
                .await?,
            4
        );
        let settled = snapshot(&h).await?;

        h.root
            .advance_market(&authority, &h.node, world)
            .await
            .map_err(|e| format!("Market expiry step: {e:?}"))?;
        assert_eq!(settled, snapshot(&h).await?);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_market_operations")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn market_postgres_buy_funds_refusal_replays_without_value_rows() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "market_refusal", true, false).await?;

        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(
                    crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                )
                .await?
                .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        let command = CommandRef::new(fence.game_session_id, CommandId::new(1).map_err(debug)?);
        let request = MarketRequest {
            action: 5,
            definition: 1,
            offer: [0; 16],
            amount: 1,
            price: 10,
            anonymous: false,
            page: 0,
        };
        let definition = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i3031".into(),
            revision_ref: "definition-r1".into(),
        };
        let first = h
            .root
            .commit_market(
                &authority,
                &h.node,
                fence,
                command,
                request,
                Some(definition.clone()),
                0,
            )
            .await?;
        assert_eq!(first.code, 2);
        let replay = h
            .root
            .commit_market(
                &authority,
                &h.node,
                fence,
                command,
                request,
                Some(definition),
                0,
            )
            .await?;
        assert_eq!(first, replay);
        for table in [
            "game_market_offers",
            "game_market_settlements",
            "game_market_item_lines",
            "game_market_event_outbox",
            "game_account_bank_balances",
        ] {
            let count: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(count, 0, "refusal wrote {table}");
        }
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
