//! S5: drop a real connection with an unfinished trade or an unread committed reply.
use super::*;
use oteryn_protocol_oteryn::{item_forge as forge, market as market_wire, player_trade as trade};

#[derive(Clone, Copy)]
enum Case {
    Trade,
    Market,
    Depot,
    Forge,
}
#[test]
fn s5_disconnect_trade_offer_cannot_be_accepted_by_retired_session() -> TestResult {
    timed("s5-trade", || scenario(Case::Trade))
}
#[test]
fn s5_disconnect_market_lost_reply_retains_one_escrow() -> TestResult {
    timed("s5-market", || scenario(Case::Market))
}
#[test]
fn s5_disconnect_depot_lost_reply_retains_one_item() -> TestResult {
    timed("s5-depot", || scenario(Case::Depot))
}
#[test]
fn s5_disconnect_forge_lost_reply_retains_one_debit() -> TestResult {
    timed("s5-forge", || scenario(Case::Forge))
}

fn scenario(case: Case) -> TestResult {
    world_scenario!("xmech_s5", |h,
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
        at_locker(&mut owner, admitted).await?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 18, 21, 37, 60, 63, 152],
        )
        .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(60));
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), session);
        owner.admit_promotion(session).await;
        assert!(owner.session_premium_gate(session).await.premium_current());
        let forge_request = forge::Request {
            operation: forge::Operation::DustLimit,
            occurrence: id(154),
            first_item: None,
            second_item: None,
            boost: false,
            protect: false,
            history_limit: 0,
        };
        let (ware_definition, ware_reference, _) = market_ware(native)?;
        let market = market_wire::MarketRequest {
            action: 6,
            definition: ware_reference,
            offer: [0; 16],
            amount: 10,
            price: 2,
            anonymous: false,
            page: 0,
        };
        let mut deposited_intent = None;
        let mut pending_trade = None;
        match case {
            Case::Trade => {
                let mut other = peer(&h, &mut owner, admitted).await?;
                s2_economy::seed_peer_bank_and_bag(&h, other).await?;
                other.continuity.selected_capabilities = admitted.continuity.selected_capabilities;
                step_west(
                    &mut owner,
                    other.runtime_actor.ok_or("peer")?,
                    other.game_session_id,
                )
                .await?;
                // Trade on widened plain grass, away from the fixture's retained map
                // gold at (7,0), whose mutable custody is outside this scenario.
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(Duration::from_secs(2))
                    .ok_or("clock")?;
                let (moved, _) = owner.paced_step(actor, session, StepDirection::East).await;
                assert_eq!(moved.disposition, StepDisposition::Moved);
                assert_eq!(position(&owner, actor).await?, TRADE_CELL);
                let leaf =
                    mint_carried(&h, &owner, admitted, "oteryn:item.tibia.i3607", 1, 155).await?;
                let peer_leaf =
                    mint_carried(&h, &owner, other, "oteryn:item.tibia.i3604", 1, 156).await?;
                let (disconnected, peer_disconnected, trade_id) =
                    trade_until_loss(&h, &owner, admitted, other, leaf, peer_leaf).await?;
                admitted = disconnected;
                pending_trade = Some((trade_id, peer_disconnected));
            }
            Case::Market => {
                // Bank/coin conversion funds the real placement fee from the fixture purse.
                super::s2_economy::deposit_bank(&h, &owner, admitted, 100).await?;
                let ware = mint_carried(
                    &h,
                    &owner,
                    admitted,
                    &ware_definition.production_key,
                    10,
                    150,
                )
                .await?;
                deposit(&h, &owner, admitted, ware, 40).await?;
                admitted = lost_reply(
                    &h,
                    &owner,
                    admitted,
                    market_wire::COMMAND_TYPE_MARKET,
                    &market_wire::encode_request(market).map_err(debug)?,
                    "SELECT count(*) FROM game_market_offers",
                )
                .await?;
                assert_eq!(
                    count(&h, "SELECT count(*) FROM game_item_market_escrow_locations").await?,
                    1
                );
                assert_eq!(
                    count(
                        &h,
                        "SELECT sum(balance)::bigint FROM game_account_bank_balances"
                    )
                    .await?,
                    80
                );
            }
            Case::Depot => {
                let gold = ledger(&h)
                    .await?
                    .backpack
                    .into_iter()
                    .find(|(key, _, _)| key == GOLD)
                    .ok_or("gold")?
                    .2;
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
                    .handle(&ItemKey::Instance(gold))
                    .ok_or("gold handle")?;
                let intent = DepotIntent {
                    town: Some(0),
                    box_number: DEPOT_BOX,
                    page: 0,
                    action: DepotAction::Deposit(handle),
                };
                deposited_intent = Some(intent);
                admitted=lost_reply(&h,&owner,admitted,COMMAND_TYPE_DEPOT_INTENT,&encode_depot_intent(intent).map_err(debug)?,"SELECT count(*) FROM game_item_loop_move_receipts WHERE depot_town=0 AND depot_box=17").await?;
                assert_eq!(
                    count(
                        &h,
                        "SELECT count(*) FROM game_item_character_depot_locations"
                    )
                    .await?,
                    1
                );
                assert_eq!(gold_in_backpack(&h).await?, 0);
            }
            Case::Forge => {
                sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING").execute(&h.pool).await?;
                let current = owner
                    .current_quest_fence(session)
                    .await
                    .map_err(|()| "fence")?
                    .ok_or("fence")?;
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                    .execute(&mut *tx)
                    .await?;
                crate::durability::character_forge_dust::gain_forge_dust_in_transaction(
                    &mut tx,
                    &current,
                    crate::durability::character_forge_dust::ForgeDustGainCause::CreatureKill {
                        occurrence: id(151),
                    },
                    100,
                    crate::durability::character_forge_dust::ForgeDustWriteIds {
                        entry_id: id(152),
                        transaction_id: id(153),
                    },
                )
                .await
                .map_err(debug)?;
                tx.commit().await?;
                admitted = lost_reply(
                    &h,
                    &owner,
                    admitted,
                    forge::COMMAND_TYPE_ITEM_FORGE,
                    &forge_request.encode().ok_or("forge encoding")?,
                    "SELECT count(*) FROM game_forge_dust_limit_receipts",
                )
                .await?;
                assert_eq!(
                    sqlx::query_as::<_, (i64, i16)>(
                        "SELECT balance,dust_limit FROM game_character_forge_dust"
                    )
                    .fetch_one(&h.pool)
                    .await?,
                    (75, 120)
                );
            }
        }
        let committed = snapshot(&h).await?;
        match case {
            Case::Trade => {}
            Case::Market => {
                let replay = owner
                    .market_command(
                        admitted,
                        1,
                        &market_wire::encode_request(market).map_err(debug)?,
                    )
                    .await
                    .ok_or("market lost-reply replay")?;
                assert_eq!(replay.get(1), Some(&1));
            }
            Case::Depot => {
                let replay = owner
                    .depot(
                        actor,
                        command(admitted, 1),
                        deposited_intent.ok_or("deposited intent")?,
                        None,
                    )
                    .await
                    .ok_or("depot lost-reply replay")?;
                assert_eq!(replay.reply.outcome, DepotOutcome::Moved);
            }
            Case::Forge => {
                let replay = owner
                    .item_forge(admitted, 1, forge_request, None)
                    .await
                    .ok_or("forge lost-reply replay")?;
                let replay: serde_json::Value = serde_json::from_slice(&replay)?;
                assert_eq!(replay["succeeded"], true);
                assert_eq!(replay["result"]["dust"], 75);
                assert_eq!(replay["result"]["dust_limit"], 120);
            }
        }
        assert_eq!(
            snapshot(&h).await?,
            committed,
            "lost-reply replay moves no value"
        );
        let before = snapshot(&h).await?;
        let at = position(&owner, actor).await?;
        if !matches!(case, Case::Trade) {
            assert_eq!(
                owner.lose_control(admitted, Duration::ZERO, None).await,
                crate::gameplay_transport::connection::ControlLossResult::Recorded
            );
        }
        assert_eq!(snapshot(&h).await?, before, "control loss moves no value");
        assert_eq!(
            owner.expire_control_loss(admitted).await,
            crate::gameplay_transport::connection::GraceExpiryResult::Released
        );
        assert_eq!(
            snapshot(&h).await?,
            before,
            "expired release moves no value"
        );
        let next = admit_successor(&h, &mut owner, admitted, at, before.clone()).await?;
        let next_actor = next.runtime_actor.ok_or("successor")?;
        match case {
            Case::Trade => {
                let (trade_id, _) = pending_trade.ok_or("pending trade")?;
                assert_eq!(
                    owner
                        .player_trade_command(
                            admitted,
                            55,
                            trade::Intent::Accept { trade_id },
                            None
                        )
                        .await,
                    Some(trade::ResultCode::Rejected)
                );
                assert_eq!(
                    count(&h, "SELECT count(*) FROM game_player_trade_receipts").await?,
                    0
                );
                assert_eq!(gold_in_backpack(&h).await?, 100);
            }
            Case::Market => {
                let stale = owner
                    .market_command(
                        admitted,
                        55,
                        &market_wire::encode_request(market).map_err(debug)?,
                    )
                    .await
                    .ok_or("stale market")?;
                assert_ne!(stale.get(1), Some(&1));
                assert_eq!(
                    count(&h, "SELECT count(*) FROM game_market_offers").await?,
                    1
                );
                assert_eq!(
                    count(&h, "SELECT count(*) FROM game_item_market_escrow_locations").await?,
                    1
                );
                assert_eq!(
                    count(
                        &h,
                        "SELECT sum(balance)::bigint FROM game_account_bank_balances"
                    )
                    .await?,
                    80
                );
            }
            Case::Depot => {
                let withdraw = DepotIntent {
                    town: Some(0),
                    box_number: DEPOT_BOX,
                    page: 0,
                    action: DepotAction::Withdraw(1),
                };
                assert_eq!(
                    owner
                        .depot(actor, command(admitted, 55), withdraw, None)
                        .await
                        .ok_or("stale depot")?
                        .reply
                        .outcome,
                    DepotOutcome::Rejected
                );
                let browse = DepotIntent {
                    action: DepotAction::Browse,
                    ..withdraw
                };
                let page = owner
                    .depot(next_actor, command(next, 1), browse, None)
                    .await
                    .ok_or("successor depot")?;
                assert_eq!(page.reply.entries.len(), 1);
                assert_eq!(page.reply.entries[0].count, 100);
            }
            Case::Forge => {
                assert_eq!(
                    owner
                        .item_forge(
                            admitted,
                            55,
                            forge::Request {
                                occurrence: id(156),
                                ..forge_request
                            },
                            None
                        )
                        .await,
                    Some(Vec::new())
                );
                assert_eq!(
                    sqlx::query_as::<_, (i64, i16)>(
                        "SELECT balance,dust_limit FROM game_character_forge_dust"
                    )
                    .fetch_one(&h.pool)
                    .await?,
                    (75, 120)
                );
                assert_eq!(
                    count(&h, "SELECT count(*) FROM game_forge_dust_limit_receipts").await?,
                    1
                );
            }
        }
        assert_eq!(
            snapshot(&h).await?,
            before,
            "no old session completes another mutation"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        drop(request);
        h.cleanup().await
    })
}
async fn gold_in_backpack(h: &Harness) -> TestResult<i64> {
    Ok(gold(&ledger(h).await?))
}

/// Keep both sockets live throughout offer/counter-offer/first consent. EOF is itself a
/// production trade cancellation, so reconnecting between offers would never test fencing.
async fn trade_until_loss(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: AdmittedSession,
    b: AdmittedSession,
    first: [u8; 16],
    second: [u8; 16],
) -> TestResult<(AdmittedSession, AdmittedSession, [u8; 16])> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    owner
        .states
        .chat
        .admit(a.game_session_id, "Fixture Hero", owner.owner_now());
    owner
        .states
        .chat
        .admit(b.game_session_id, "Scenario Peer", owner.owner_now());
    let handle = async |admitted: AdmittedSession, item: [u8; 16]| -> TestResult<u64> {
        let mut view = SessionItemView::resume(ItemViewContinuity::default());
        view.snapshot(
            owner
                .observe_character_inventory(
                    admitted.runtime_actor.ok_or("actor")?,
                    admitted.game_session_id,
                )
                .await
                .ok_or("inventory")?,
            None,
        )
        .map_err(debug)?;
        Ok(view
            .table()
            .handle(&ItemKey::Instance(item))
            .ok_or("handle")?
            .get())
    };
    let first_handle = handle(a, first).await?;
    let second_handle = handle(b, second).await?;
    let targets = {
        let r = owner.runtime.lock().await;
        let entities = r.visible_entities();
        let entity = |admitted: AdmittedSession| -> TestResult<EntityRef> {
            let p = entities
                .players
                .iter()
                .find(|p| Some(p.actor) == admitted.runtime_actor)
                .ok_or("visible player")?;
            Ok(EntityRef {
                identity: p.actor.placement_identity(),
                generation: p.generation,
            })
        };
        [entity(a)?, entity(b)?]
    };
    let (mut server_a, mut client_a) = tokio::io::duplex(1 << 20);
    let (mut server_b, mut client_b) = tokio::io::duplex(1 << 20);
    let serving_a = async move {
        let end = serve_admitted(&mut server_a, a, owner, IDLE_LIVENESS).await;
        drop(server_a);
        end
    };
    let serving_b = async move {
        let end = serve_admitted(&mut server_b, b, owner, IDLE_LIVENESS).await;
        drop(server_b);
        end
    };
    let driving = async {
        ready(&mut client_a).await?;
        ready(&mut client_b).await?;
        let before = snapshot(h).await?;
        assert_eq!(
            trade_send(
                &mut client_a,
                1,
                trade::Intent::Offer {
                    partner: targets[1],
                    item: first_handle
                }
            )
            .await?,
            trade::ResultCode::Offered.payload()
        );
        assert_eq!(snapshot(h).await?, before);
        assert_eq!(
            trade_send(
                &mut client_b,
                1,
                trade::Intent::Offer {
                    partner: targets[0],
                    item: second_handle
                }
            )
            .await?,
            trade::ResultCode::Offered.payload()
        );
        assert_eq!(snapshot(h).await?, before);
        let trade_id = owner
            .trade_observation(a.game_session_id)
            .await
            .ok_or("ready trade")?
            .1
            .trade_id
            .ok_or("trade id")?;
        assert_eq!(
            trade_send(&mut client_a, 2, trade::Intent::Accept { trade_id }).await?,
            trade::ResultCode::Accepted.payload()
        );
        assert_eq!(
            count(h, "SELECT count(*) FROM game_player_trade_receipts").await?,
            0
        );
        assert_eq!(snapshot(h).await?, before);
        drop(client_a);
        assert_eq!(
            owner.lose_control(a, Duration::ZERO, None).await,
            crate::gameplay_transport::connection::ControlLossResult::Recorded
        );
        let late = trade_send(&mut client_b, 2, trade::Intent::Accept { trade_id }).await?;
        assert_ne!(late, trade::ResultCode::Completed.payload());
        assert_ne!(late, trade::ResultCode::Accepted.payload());
        assert_eq!(
            count(h, "SELECT count(*) FROM game_player_trade_receipts").await?,
            0
        );
        assert_eq!(
            snapshot(h).await?,
            before,
            "lost controller cannot complete its frozen consent"
        );
        drop(client_b);
        Ok::<_, Box<dyn std::error::Error>>(trade_id)
    };
    let ((end_a, end_b), result) = super::super::fixture::join_two(
        super::super::fixture::join_two(serving_a, serving_b),
        driving,
    )
    .await;
    let admitted = |end| -> TestResult<AdmittedSession> {
        match end {
            ConnectionEnd::AdmittedThenDisconnected(next)
            | ConnectionEnd::AdmittedThenClosed(next, _)
            | ConnectionEnd::AdmittedThenControlLost(next) => Ok(next),
            other => Err(format!("trade socket: {other:?}").into()),
        }
    };
    Ok((admitted(end_a)?, admitted(end_b)?, result?))
}
async fn ready(client: &mut tokio::io::DuplexStream) -> TestResult {
    loop {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(client).await?;
        if oteryn_protocol_oteryn::decode_wire_envelope(&frame)
            .map_err(debug)?
            .message_type()
            == oteryn_protocol_oteryn::MessageType::SnapshotCommit
        {
            return Ok(());
        }
    }
}
async fn trade_send(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    intent: trade::Intent,
) -> TestResult<Vec<u8>> {
    let payload = trade::encode_intent(intent).map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: id,
            command_type: trade::COMMAND_TYPE_PLAYER_TRADE_INTENT,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    crate::gameplay_transport::tcp_tls::write_frame(client, &frame).await?;
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let frame = crate::gameplay_transport::tcp_tls::read_frame(client).await?;
            let env = oteryn_protocol_oteryn::decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                let result =
                    oteryn_protocol_oteryn::decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.command_id, id);
                return Ok::<_, Box<dyn std::error::Error>>(result.payload.to_vec());
            }
        }
    })
    .await?
}
