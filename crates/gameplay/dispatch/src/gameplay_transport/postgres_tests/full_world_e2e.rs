//! E2E-WORLD-1: one Character's playable chain on one bundle World through the composed owner
//! and PostgreSQL: admission, paced steps, an NPC purchase, an auto-attack kill with its loot, a
//! depot deposit over the real connection, logout and a successor admission. After the relogin
//! the durable gold, items, depot and position are exactly what the chain committed, and every
//! replayed step answers from its stored outcome without moving value a second time.
use super::full_world_support::*;

#[test]
fn full_world_chain_survives_relogin_and_replays_move_no_value_on_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("full-world-e2e", scenario)
}

fn scenario() -> TestResult {
    world_scenario!(
        "full_world_e2e",
        |h,
         request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            let purse = ledger(&h).await?;
            assert_eq!(gold(&purse), 100, "{purse:?}");

            // 2. Move: two paced West steps through the real step owner on the bundle cells.
            let mut at = step_west(&mut owner, actor, session).await?;
            at = step_west(&mut owner, actor, session)
                .await
                .map_err(|e| format!("{at:?}: {e}"))?;
            assert_eq!(at, TRADE_CELL);

            // 3. Buy from an NPC: the production NPC book, a placed NPC beside the trade cell,
            // NPC_TALK "hi" then "trade" opens the window, NPC_TRADE buys its first Buy offer.
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
            owner
                .states
                .chat
                .admit(session, "Fixture Hero", owner.owner_now());
            owner.states.chat.set_npc_service(session);
            let npc = {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .place_npcs(
                        &NpcPlacementTable {
                            npcs: vec![NpcPlacements {
                                key: NPC.into(),
                                placements: vec![NpcPlacement {
                                    floor: -7,
                                    x: NPC_CELL.0,
                                    y: NPC_CELL.1,
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
            let use_command = |command_id| UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence),
            };
            let mut window = None;
            for (command_id, text) in [(10, "hi"), (11, "trade")] {
                let outcome = owner
                    .npc_talk(
                        actor,
                        use_command(command_id),
                        NpcTalkIntent {
                            npc_actor: npc,
                            text: text.into(),
                        },
                    )
                    .await;
                assert!(!outcome.unknown, "{text}");
                assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
                window = outcome.window;
            }
            let window = window.ok_or("the trade window")?;
            let (_, revision) = window.open.ok_or("an open trade window")?;
            let mut bought = None;
            for (offer_index, offer) in window.offers.iter().enumerate() {
                if offer.side != NpcTradeSide::Buy || u64::from(offer.unit_price) > 100 {
                    continue;
                }
                let intent = NpcTradeIntent {
                    npc_actor: npc,
                    catalogue_revision: revision,
                    offer_index: u32::try_from(offer_index)?,
                    side: NpcTradeSide::Buy,
                    quantity: 1,
                    expected_unit_price: offer.unit_price,
                };
                let command_id = 12 + u64::try_from(offer_index)?;
                if let NpcTradeStep::Traded = owner
                    .npc_trade(actor, use_command(command_id), intent)
                    .await
                {
                    bought = Some((command_id, intent, *offer));
                    break;
                }
            }
            let (trade_command, trade_intent, offer) = bought.ok_or("a Buy offer trades")?;
            let traded = ledger(&h).await?;
            assert_eq!(traded.npc_trades, 1, "{traded:?}");
            assert_eq!(
                gold(&traded),
                100 - i64::from(offer.unit_price),
                "the purchase charged its unit price once"
            );
            let item = traded
                .backpack
                .iter()
                .find(|(key, _, id)| {
                    key != GOLD && !purse.backpack.iter().any(|(_, _, seen)| seen == id)
                })
                .map(|(_, _, id)| *id)
                .ok_or("the bought item is in the backpack")?;
            // Replay: the same command and intent answer from the stored trade.
            assert!(matches!(
                owner
                    .npc_trade(actor, use_command(trade_command), trade_intent)
                    .await,
                NpcTradeStep::Traded
            ));
            assert_eq!(
                ledger(&h).await?,
                traded,
                "an NPC trade replay moves nothing"
            );

            // 4. Kill a creature and loot it: a production rat beside the trade cell, held as the
            // auto-attack target and driven by the real swing and reward drains.
            let rat = {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .realize_native_qualification_spawn(
                        pin.clone(),
                        "oteryn:creature.rat",
                        RAT_CELL,
                    )
                    .map_err(|e| format!("rat spawn: {e:?}"))?;
                let creature = runtime
                    .visible_entities()
                    .creatures
                    .first()
                    .copied()
                    .ok_or("the rat is visible")?;
                EntityRef {
                    identity: creature.actor.placement_identity(),
                    generation: creature.generation,
                }
            };
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
                    .checked_sub(Duration::from_secs(3))
                    .ok_or("owner clock")?;
            }
            let mut drains = 0;
            while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
                drains += 1;
                assert!(drains < 8, "the queued kill settles");
            }
            let killed = ledger(&h).await?;
            // 5 rat XP, doubled by the level-1 Global low-level bonus.
            assert_eq!((killed.xp_receipts, killed.experience), (1, 10));
            assert_eq!(killed.corpse_mints, 1);
            // Replay of the settlement: the drained queue settles nothing again.
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            assert_eq!(ledger(&h).await?, killed, "a drained kill settles once");
            let corpse = owner
                .runtime
                .lock()
                .await
                .visible_corpses()
                .into_iter()
                .next()
                .ok_or("the kill's visible corpse")?;
            let contents = h
                .root
                .read_corpse_contents(&authority, corpse.item.item_instance_id)
                .await
                .map_err(debug)?
                .ok_or("corpse contents")?;
            let loot = contents.first().ok_or("the rat's corpse carries loot")?;
            let (loot_id, loot_quantity) = (loot.item.item_instance_id, loot.item.quantity);
            owner
                .take_corpse_entry(
                    actor,
                    use_command(20),
                    ItemKey::Entity(corpse.identity),
                    ItemKey::Instance(loot_id),
                )
                .await
                .map_err(|e| format!("loot take: {e:?}"))?;
            let looted = ledger(&h).await?;
            assert_eq!(looted.corpse_entries, killed.corpse_entries - 1);
            assert_eq!(
                looted.backpack.iter().map(|(_, q, _)| q).sum::<i64>(),
                killed.backpack.iter().map(|(_, q, _)| q).sum::<i64>() + i64::from(loot_quantity),
                "the loot entry moved into the backpack once"
            );
            let _ = owner
                .take_corpse_entry(
                    actor,
                    use_command(20),
                    ItemKey::Entity(corpse.identity),
                    ItemKey::Instance(loot_id),
                )
                .await;
            assert_eq!(ledger(&h).await?, looted, "a loot replay moves nothing");

            // 5. Deposit at the depot over the real connection: step beside the locker, then
            // DEPOT_INTENT browse and deposit of the bought item through `serve_admitted`.
            assert_eq!(step_west(&mut owner, actor, session).await?, LOCKER_CELL);
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37])
                    .ok_or("selected capabilities")?;
            let mut view = SessionItemView::resume(ItemViewContinuity::default());
            view.snapshot(
                owner
                    .observe_character_inventory(actor, session)
                    .await
                    .ok_or("inventory")?,
                None,
            )
            .map_err(debug)?;
            let handle = view
                .table()
                .handle(&ItemKey::Instance(item))
                .ok_or("bought item handle")?;
            let browse = DepotIntent {
                town: None,
                box_number: DEPOT_BOX,
                page: 0,
                action: DepotAction::Browse,
            };
            let deposit = DepotIntent {
                town: Some(0),
                action: DepotAction::Deposit(handle),
                ..browse
            };
            let mut input = Vec::new();
            for (command_id, intent) in [(1, browse), (2, deposit)] {
                let payload = encode_depot_intent(intent).map_err(debug)?;
                let frame = oteryn_protocol_oteryn::encode_client_command(
                    1,
                    &oteryn_protocol_oteryn::ClientCommandValue {
                        command_id,
                        command_type: COMMAND_TYPE_DEPOT_INTENT,
                        payload: &payload,
                    },
                )
                .map_err(debug)?;
                input.extend(u32::try_from(frame.len())?.to_be_bytes());
                input.extend(frame);
            }
            let (end, output) =
                super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
            let mut replies = Vec::new();
            let mut cursor = output.as_slice();
            while !cursor.is_empty() {
                let length =
                    u32::from_be_bytes(cursor.get(..4).ok_or("frame length")?.try_into()?) as usize;
                let frame = cursor.get(4..4 + length).ok_or("frame")?;
                cursor = &cursor[4 + length..];
                let envelope =
                    oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
                if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(debug)?;
                    replies.push((
                        result.status,
                        decode_depot_reply(result.payload).map_err(debug)?.outcome,
                    ));
                }
            }
            assert_eq!(
                replies,
                [
                    (
                        oteryn_protocol_oteryn::CommandStatus::Accepted,
                        DepotOutcome::Opened
                    ),
                    (
                        oteryn_protocol_oteryn::CommandStatus::Accepted,
                        DepotOutcome::Moved
                    )
                ],
                "{end}"
            );
            let deposited = ledger(&h).await?;
            assert_eq!(deposited.depot_receipts, 1);
            assert_eq!(deposited.depot_counter, Some(("2".into(), 1)));
            assert!(
                !deposited.backpack.iter().any(|(_, _, id)| *id == item),
                "the deposited item left the backpack"
            );
            assert_eq!(deposited.live_items, looted.live_items);
            assert_eq!(deposited.live_quantity, looted.live_quantity);
            // Replay of the deposit with the connection's outcome cache gone: durable, no second move.
            let replay = owner
                .depot(actor, use_command(2), deposit, None)
                .await
                .ok_or("durable depot replay")?
                .reply;
            assert_eq!(replay.outcome, DepotOutcome::Moved);
            assert_eq!(ledger(&h).await?, deposited, "a depot replay moves nothing");

            // 6. Logout: the terminal release writes the final cell; a replay is TERMINAL and a
            // second logout finds no actor. Out of fight first: the owner clock moves past it.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(120))
                .ok_or("owner clock")?;
            assert_eq!(
                owner.logout_session(admitted).await,
                LogoutReply::Result(LogoutResult::Accepted)
            );
            let stored: (i32, i32, i16) = sqlx::query_as(
                "SELECT tile_x, tile_y, tile_floor FROM game_character_last_positions \
             WHERE character_id = encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(stored, (7, 0, 7));
            let replay = FreshAdmissionStore::from_root(h.root.clone())
                .release_logout_session(
                    session,
                    &crate::gameplay_transport::canonical_uuid(&id(40)),
                    admitted.controller.ok_or("controller")?.transport,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(replay, ExpiredLossReleaseV1::Terminal));
            assert_eq!(
                owner.logout_session(admitted).await,
                LogoutReply::Result(LogoutResult::Busy)
            );
            let logged_out = ledger(&h).await?;
            assert_eq!(logged_out, deposited, "logout moves no value");

            // 7. Relogin: the successor fresh admission of the same Character, then its admission
            // position from the durable last position through the production owner steps.
            let next_session = relogin(&h).await?;
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let current = store.current_session(next_session).await?;
            assert_eq!(current.current_character_lease().generation(), 3);
            let next_fence =
                crate::gameplay_transport::item_fence_of(current).ok_or("next fence")?;
            let next_actor = {
                let mut runtime = deps.runtime.lock().await;
                let reservation = runtime.reserve_fresh_session(next_session).map_err(debug)?;
                runtime.commit_fresh_session(reservation).map_err(debug)?
            };
            let gameplay_fence = owner
                .current_quest_fence(next_session)
                .await
                .map_err(|()| "successor fence")?
                .ok_or("successor session")?;
            let decided = owner
                .read_admission_position(gameplay_fence)
                .await
                .map_err(|()| "admission position")?;
            assert!(
                matches!(decided, AdmissionPosition::Stored(_)),
                "the relogin reads the stored last position"
            );
            {
                let mut runtime = deps.runtime.lock().await;
                owner
                    .initialize_admitted_position(&mut runtime, next_actor, next_session, &decided)
                    .await
                    .map_err(debug)?;
            }
            assert!(owner.track_restored(next_session, next_actor, &decided, gameplay_fence));
            let mut successor = crate::gameplay_transport::connection::AdmittedSession {
                game_session_id: next_session,
                runtime_actor: Some(next_actor),
                item_fence: Some(next_fence),
                ..admitted
            };
            owner.admit_loop_training(&mut successor).await;
            assert_eq!(successor.first_entry, FirstEntryOutcome::Positioned);
            let markers: (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE cause='offline_settlement'), count(*) FILTER (WHERE cause='logout') FROM game_character_training_markers",
        )
        .fetch_one(&h.pool)
        .await?;
            assert_eq!(
                markers,
                (2, 1),
                "default training survives logout and relogin"
            );
            assert_eq!(position(&owner, next_actor).await?, LOCKER_CELL);
            let relogged = ledger(&h).await?;
            assert_eq!(relogged, deposited, "relogin moves no value");
            let depot_view = owner
                .depot(
                    next_actor,
                    UseCommand {
                        game_session_id: next_session,
                        command_id: 1,
                        item_fence: Some(next_fence),
                    },
                    DepotIntent {
                        town: Some(0),
                        ..browse
                    },
                    None,
                )
                .await
                .ok_or("successor depot browse")?
                .reply;
            assert_eq!(depot_view.outcome, DepotOutcome::Opened);
            assert_eq!(depot_view.entries.len(), 1);
            assert_eq!(depot_view.entries[0].count, 1);
            assert_eq!(ledger(&h).await?, deposited, "browsing moves no value");
            drop(owner);
            drop(authority);
            drop(seal);

            // A reopened Character authority reads the same backpack the chain left.
            let seal = h.recovery.seal_current().map_err(debug)?;
            let reopened = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let reloaded = h
                .root
                .read_character_backpack(&reopened, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("backpack reload")?;
            let mut durable = reloaded
                .entries
                .iter()
                .map(|entry| (entry.item.quantity, entry.item.item_instance_id))
                .collect::<Vec<_>>();
            durable.sort();
            let mut expected = deposited
                .backpack
                .iter()
                .map(|(_, quantity, id)| Ok((u32::try_from(*quantity)?, *id)))
                .collect::<TestResult<Vec<_>>>()?;
            expected.sort();
            assert_eq!(durable, expected);
            drop(reopened);
            drop(seal);
            drop(request);
            h.cleanup().await
        }
    )
}
