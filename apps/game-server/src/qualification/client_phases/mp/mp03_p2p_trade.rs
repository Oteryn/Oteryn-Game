//! MP03: two traders and a third busy-trade contender, all on real client sessions.
//! Reference (OTS_HYPOTHESIS_ONLY): Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! src/game/game.cpp:5217 (offer/range), 5359 (offer disclosure), 5393 (both accepts), 5594 (cancel both).
//! No provisioning: use the same ordinary starter/profile inventory as MP01.
use super::*;
use bots::Bot;
use oteryn_dev_client::client_runtime::player_trade as runtime;
use oteryn_protocol_oteryn::{item_view::ItemEntry, player_trade as wire};

const CASES: [&str; 9] = [
    "admissions",
    "stack",
    "approach",
    "cancel",
    "offers",
    "first-accept",
    "exchange",
    "not-owned",
    "relog",
];
type Inventory = Vec<(u32, u32, u32)>;

fn report(case: &str, result: &TestResult) {
    match result {
        Ok(()) => evidence(&format!("MP_CHECK MP03 {case} PASS verified")),
        Err(error) => {
            verdict::record_assertion(case);
            evidence(&format!(
                "MP_CHECK MP03 {case} FAIL {error};source:{}:{}",
                file!(),
                line!()
            ));
        }
    }
}

async fn row<T>(
    case: &'static str,
    action: impl std::future::Future<Output = TestResult<T>>,
    completed: &mut usize,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, case);
    let result = tokio::time::timeout_at(deadline, action)
        .await
        .map_err(|_| {
            format!(
                "MP03 {case} deadline;server_reply=none_received_before_deadline;source:{}:{}",
                file!(),
                line!()
            )
        })
        .and_then(|result| result.map_err(|error| error.to_string()));
    report(
        case,
        &result.as_ref().map(|_| ()).map_err(|e| e.clone().into()),
    );
    *completed += 1;
    result.map_err(Into::into)
}

pub(super) async fn run(
    core: &SeamClients<'_>,
    packets: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let started = std::time::Instant::now();
    // On admission timeout the partial pool drops its sockets; release_all still checks all four Characters.
    let (mut players, setup) = tokio::time::timeout(
        Duration::from_secs(180),
        bots::connect_bots(&[(core, 0, None), (packets, 0, None), (packets, 1, None)]),
    )
    .await
    .unwrap_or_else(|_| {
        (
            Vec::new(),
            Err("MP03 admissions deadline;server_reply=none_received_before_deadline".into()),
        )
    });
    report("admissions", &setup);
    let mut completed = 1;
    let result = match setup {
        Ok(()) => journey(&mut players, world, &mut completed).await,
        Err(error) => Err(error),
    };
    if let Err(error) = &result {
        for case in &CASES[completed..] {
            report(
                case,
                &Err(format!("dependent check blocked;reply/cause={error}").into()),
            );
        }
    }
    let mut refusal_failures = Vec::new();
    if players.len() == 3 && players.iter().all(|bot| bot.session.is_some()) {
        for case in ["out-of-range", "already-trading", "receiver-full"] {
            let result = row(case, refusal(&mut players, world, case), &mut completed).await;
            if let Err(error) = result {
                refusal_failures.push(error.to_string());
            }
            let reset = row("reset", reset_trades(&mut players), &mut completed).await;
            if let Err(error) = reset {
                refusal_failures.push(error.to_string());
            }
        }
    } else {
        for case in ["out-of-range", "already-trading", "receiver-full"] {
            let failure = format!("{case} admissions unavailable;reply/cause={result:?}");
            report(case, &Err(failure.clone().into()));
            refusal_failures.push(failure);
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1800);
    verdict::Harness::global().bounded_wait(deadline, "MP03-release");
    let cleanup = tokio::time::timeout_at(deadline, bots::release_all(&mut players, core, packets))
        .await
        .map_err(|_| "MP03 release deadline;server_reply=none_received_before_deadline".into())
        .and_then(|r| r);
    // Any poisoned/remaining sockets also close on a cleanup deadline.
    for player in &mut players {
        player.session = None;
    }
    report("release", &cleanup);
    let mut failures: Vec<_> = [result, cleanup]
        .into_iter()
        .filter_map(Result::err)
        .map(|e| e.to_string())
        .collect();
    failures.extend(refusal_failures);
    evidence(&format!(
        "e2e_all scenario=MP03.full status={} seconds={:.3}",
        if failures.is_empty() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; ").into())
    }
}

fn inventory(bot: &mut Bot<'_, '_>) -> TestResult<Inventory> {
    let view = bot
        .client()?
        .inventory()
        .ok_or("MP03 inventory snapshot absent")?;
    let mut items: Vec<_> = view
        .entries
        .iter()
        .map(|e| (e.item_definition_ref.get(), e.count.get(), e.sub_type))
        .collect();
    items.sort_unstable();
    Ok(items)
}

async fn send(
    bot: &mut Bot<'_, '_>,
    intent: wire::Intent,
    expected: wire::ResultCode,
) -> TestResult<wire::State> {
    let command = runtime::PlayerTradeCommand::new(intent).ok_or("MP03 invalid trade intent")?;
    let reply = runtime::send(bot.client()?.session_mut(), &command).await?;
    evidence(&format!("MP03_REPLY intent={intent:?} reply={reply:?}"));
    if reply.status != CommandStatus::Accepted || reply.result != expected {
        return Err(format!("reply={reply:?};intent={intent:?};source:crates/gameplay/dispatch/src/gameplay_transport/player_trade/command.rs:37").into());
    }
    Ok(reply.state)
}

async fn update(bot: &mut Bot<'_, '_>) -> TestResult<wire::State> {
    let delta = bot
        .client()?
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_PLAYER_TRADE, 1)
        .await?;
    wire::decode_state(&delta.payload)
        .map_err(|e| format!("MP03 state decode={e:?};reply={delta:?}").into())
}

fn entry_offer(item: ItemEntry) -> wire::Offer {
    wire::Offer {
        definition: item.item_definition_ref.get(),
        count: item.count.get(),
        sub_type: item.sub_type,
    }
}

async fn offer(
    players: &mut [Bot<'_, '_>],
    side: usize,
    item: ItemEntry,
) -> TestResult<[wire::State; 2]> {
    let peer = 1 - side;
    let partner = players[peer].own()?;
    let own = send(
        &mut players[side],
        wire::Intent::Offer {
            partner,
            item: item.handle.get(),
        },
        wire::ResultCode::Offered,
    )
    .await?;
    let other = update(&mut players[peer]).await?;
    // Crystal game.cpp:5376-5387 discloses the counter item only after both offer.
    if own.trade_id != other.trade_id
        || own.own != Some(entry_offer(item))
        || other.counter != other.own.and(own.own)
        || own.partner != Some(partner)
        || other.partner != Some(players[side].own()?)
    {
        return Err(format!("MP03 asymmetric offers;reply={own:?};peer_reply={other:?}").into());
    }
    Ok(if side == 0 {
        [own, other]
    } else {
        [other, own]
    })
}

async fn receipts(players: &[Bot<'_, '_>]) -> TestResult<Option<String>> {
    let mut database = sqlx::PgConnection::connect(players[0].pair.url).await?;
    let id = sqlx::query_scalar("SELECT occurrence_id::text FROM game_player_trade_participants WHERE character_id=encode($1,'hex')::uuid ORDER BY created_xact_id DESC,occurrence_id DESC LIMIT 1")
        .bind(players[0].character.as_slice()).fetch_optional(&mut database).await?;
    database.close().await?;
    Ok(id)
}

async fn durable(players: &[Bot<'_, '_>], previous: &Option<String>) -> TestResult {
    let current = receipts(players).await?;
    if current.is_none() || &current == previous {
        return Err(format!(
            "MP03 no fresh durable exchange;reply_receipt={current:?};before={previous:?}"
        )
        .into());
    }
    let mut database = sqlx::PgConnection::connect(players[0].pair.url).await?;
    // Exactly two receipt rows; each original instance must now be in the opposite backpack.
    let rows: Vec<(String, String, bool)> = sqlx::query_as("SELECT r.source_character_id::text,r.destination_character_id::text, COALESCE(e.character_id=r.destination_character_id AND e.parent_item_instance_id=r.destination_parent,false) FROM game_player_trade_receipts r LEFT JOIN game_item_container_entries e USING(item_instance_id) WHERE r.occurrence_id=$1::text::uuid ORDER BY r.side LIMIT 3")
        .bind(&current).fetch_all(&mut database).await?;
    database.close().await?;
    let a = uuid_text(&players[0].character);
    let b = uuid_text(&players[1].character);
    if rows.len() != 2
        || !rows.contains(&(a.clone(), b.clone(), true))
        || !rows.contains(&(b, a, true))
    {
        return Err(format!(
            "MP03 durable ownership mismatch;receipt={current:?};reply_rows={rows:?}"
        )
        .into());
    }
    Ok(())
}

async fn journey(
    players: &mut [Bot<'_, '_>],
    world: &BundleWorld,
    completed: &mut usize,
) -> TestResult {
    row("stack", acquire_stack(players, world), completed).await?;
    row(
        "approach",
        bots::approach(players, 0, 1, world, false),
        completed,
    )
    .await?;
    let before = [inventory(&mut players[0])?, inventory(&mut players[1])?];
    let items = players[..2].iter_mut().map(|bot| {
        bot.client()?.inventory().and_then(|v| v.entries.iter().find(|e| e.count.get() > 1).or_else(|| v.entries.first())).copied().ok_or_else(|| "MP03 no carried leaf;source:crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs:373".into())
    }).collect::<TestResult<Vec<_>>>()?;
    let previous = tokio::time::timeout(Duration::from_secs(10), receipts(players))
        .await
        .map_err(|_| "MP03 initial receipt read deadline;server_reply=none")??;
    row("cancel", async {
        let states = offer(players, 0, items[0]).await?;
        let cancelled = send(&mut players[1], wire::Intent::Cancel, wire::ResultCode::Cancelled).await?;
        let peer = update(&mut players[0]).await?;
        if cancelled.phase != 4 || peer.phase != 4 || cancelled.reason != 2 || peer.reason != 2
            || inventory(&mut players[0])? != before[0] || inventory(&mut players[1])? != before[1]
            || receipts(players).await? != previous {
            return Err(format!("MP03 cancellation changed ownership or left trade open;reply={cancelled:?};peer_reply={peer:?};offered={states:?}").into());
        }
        Ok(())
    }, completed).await?;
    let states = row(
        "offers",
        async {
            offer(players, 0, items[0]).await?;
            let states = offer(players, 1, items[1]).await?;
            if states[0].counter != Some(entry_offer(items[1]))
                || states[1].counter != Some(entry_offer(items[0]))
                || states.iter().any(|state| {
                    state.phase != 2 || state.accepted != [false; 2] || state.trade_id.is_none()
                })
            {
                return Err(format!("MP03 ready views disagree;reply={states:?}").into());
            }
            Ok(states)
        },
        completed,
    )
    .await?;
    let trade_id = states[0].trade_id.ok_or("MP03 ready trade id absent")?;
    row("first-accept", async {
        let accepted = send(&mut players[0], wire::Intent::Accept { trade_id }, wire::ResultCode::Accepted).await?;
        let peer = update(&mut players[1]).await?;
        if accepted.accepted != [true, false] || peer.accepted != [false, true]
            || inventory(&mut players[0])? != before[0] || inventory(&mut players[1])? != before[1]
            || receipts(players).await? != previous {
            return Err(format!("MP03 first accept transferred early or views disagree;reply={accepted:?};peer_reply={peer:?}").into());
        }
        Ok(())
    }, completed).await?;
    let expected = [
        exchanged(&before[0], items[0], items[1])?,
        exchanged(&before[1], items[1], items[0])?,
    ];
    row("exchange", async {
        let closed = send(&mut players[1], wire::Intent::Accept { trade_id }, wire::ResultCode::Completed).await?;
        let peer = update(&mut players[0]).await?;
        if closed.phase != 4 || peer.phase != 4 || closed.reason != 1 || peer.reason != 1 {
            return Err(format!("MP03 completion not seen by both players;reply={closed:?};peer_reply={peer:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            bots::pump(players).await?;
            let views = [inventory(&mut players[0])?, inventory(&mut players[1])?];
            if views == expected { break; }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("MP03 inventory exchange mismatch;reply={closed:?};peer_reply={peer:?};views={views:?};expected={expected:?}").into());
            }
        }
        durable(players, &previous).await?;
        let committed = receipts(players).await?;
        let replay = send(&mut players[0], wire::Intent::Accept { trade_id }, wire::ResultCode::Stale).await?;
        if receipts(players).await? != committed || inventory(&mut players[0])? != expected[0] || inventory(&mut players[1])? != expected[1] {
            return Err(format!("MP03 repeated accept transferred twice;reply={replay:?}").into());
        }
        Ok(())
    }, completed).await?;
    row(
        "not-owned",
        async {
            let committed = receipts(players).await?;
            let partner = players[1].own()?;
            let refused = send(
                &mut players[0],
                wire::Intent::Offer {
                    partner,
                    item: items[0].handle.get(),
                },
                wire::ResultCode::NotTradeable,
            )
            .await?;
            if inventory(&mut players[0])? != expected[0]
                || inventory(&mut players[1])? != expected[1]
                || receipts(players).await? != committed
            {
                return Err(
                    format!("MP03 not-owned offer moved inventory;reply={refused:?}").into(),
                );
            }
            Ok(())
        },
        completed,
    )
    .await?;
    row(
        "relog",
        async {
            let sessions = [
                players[0].client()?.join_snapshot().game_session_id,
                players[1].client()?.join_snapshot().game_session_id,
            ];
            bots::relog_all(players).await?;
            for side in 0..2 {
                if players[side].client()?.join_snapshot().game_session_id == sessions[side]
                    || inventory(&mut players[side])? != expected[side]
                {
                    return Err(format!(
                        "MP03 relog lost exchanged inventory;side={side};reply={:?}",
                        players[side].client()?.inventory()
                    )
                    .into());
                }
            }
            durable(players, &previous).await
        },
        completed,
    )
    .await
}

fn exchanged(
    before: &Inventory,
    outgoing: ItemEntry,
    incoming: ItemEntry,
) -> TestResult<Inventory> {
    let mut after = before.clone();
    let outgoing = entry_offer(outgoing);
    let index = after
        .iter()
        .position(|e| *e == (outgoing.definition, outgoing.count, outgoing.sub_type))
        .ok_or("MP03 outgoing item absent")?;
    after.remove(index);
    let incoming = entry_offer(incoming);
    after.push((incoming.definition, incoming.count, incoming.sub_type));
    after.sort_unstable();
    Ok(after)
}

/// Prerequisites are ordinary NPC purchases/sales, with live quotes, never fixture gold.
async fn shop(
    players: &mut [Bot<'_, '_>],
    side: usize,
    world: &BundleWorld,
) -> TestResult<oteryn_protocol_oteryn::npc_service::NpcTradeWindow> {
    use oteryn_dev_client::EntityKind;
    use oteryn_protocol_oteryn::npc_service as npc;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let merchant = metadata
        .npcs
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.sam")
        // The bundle NPC table holds only placed NPCs; Sam is not in the placement scope.
        .ok_or("MP03 CONTENT_GAP compiled Sam NPC absent; oteryn:npc.sam absent from tools/content-schema/npc-authoring/npc_placement_scope.json:4 npcs, so content/world/npc-placements has no Sam record")?;
    let from = tile(players[side].client()?.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        merchant
            .placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or("MP03 no route to Sam")?;
    bots::walk_bot(players, side, world, &path).await?;
    let actor = players[side]
        .client()?
        .world_entities()
        .and_then(|view| {
            view.iter().find(|e| {
                e.kind == EntityKind::Npc
                    && merchant.placements.iter().any(|p| {
                        e.position.x == i32::from(p.x)
                            && e.position.y == i32::from(p.y)
                            && e.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|e| e.entity)
        .ok_or("MP03 Sam not visible after walk")?;
    for text in ["hi", "trade"] {
        let reply = oteryn_dev_client::client_runtime::npc_shop::talk(
            players[side].client()?.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("MP03 prerequisite dialogue={text};reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/npc_service.rs:1").into());
        }
        players[side]
            .client()?
            .session_mut()
            .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
    }
    let delta = players[side]
        .client()?
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    npc::decode_npc_trade_window(&delta.payload)
        .map_err(|e| format!("MP03 shop decode={e:?};reply={delta:?}").into())
}

async fn shop_trade(
    players: &mut [Bot<'_, '_>],
    side: usize,
    window: &oteryn_protocol_oteryn::npc_service::NpcTradeWindow,
    index: usize,
) -> TestResult<oteryn_protocol_oteryn::npc_service::NpcIntentDisposition> {
    use oteryn_protocol_oteryn::npc_service as npc;
    let (npc_actor, catalogue_revision) = window
        .open
        .ok_or_else(|| format!("MP03 shop closed;reply={window:?}"))?;
    let offer = window.offers[index];
    let reply = oteryn_dev_client::client_runtime::npc_shop::trade(
        players[side].client()?.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor,
            catalogue_revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    evidence(&format!(
        "MP03_SHOP side={side} offer={offer:?} reply={reply:?}"
    ));
    bots::pump(players).await?;
    Ok(reply)
}

async fn acquire_stack(players: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult {
    if players[0]
        .client()?
        .inventory()
        .is_some_and(|v| v.entries.iter().any(|e| e.count.get() > 1))
    {
        return Ok(());
    }
    let window = shop(players, 0, world).await?;
    let carried = players[0]
        .client()?
        .inventory()
        .cloned()
        .ok_or("MP03 inventory absent")?;
    let (index, _) = window.offers.iter().enumerate().find(|(_, offer)| offer.side == oteryn_protocol_oteryn::npc_service::NpcTradeSide::Sell && offer.unit_price > 1 && carried.entries.iter().any(|e| e.item_definition_ref.get() == offer.item_definition_ref)).ok_or_else(|| format!("MP03 no live buyer for carried items;reply={window:?};inventory={carried:?};source:apps/game-server/src/node/serve/qualification_fixture.rs:242"))?;
    let reply = shop_trade(players, 0, &window, index).await?;
    if reply != oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok {
        return Err(format!("MP03 sale refused;reply={reply:?};source:crates/gameplay/dispatch/src/gameplay_transport/npc_service.rs:1").into());
    }
    bots::wait_observation(
        players,
        tokio::time::Instant::now() + Duration::from_secs(10),
        &format!("NPC sale reply={reply:?};no stack inventory delta"),
        |bots, _| {
            Ok(bots[0]
                .session
                .as_ref()
                .and_then(|s| s.inventory())
                .is_some_and(|v| v.entries.iter().any(|e| e.count.get() > 1)))
        },
    )
    .await?;
    Ok(())
}

async fn reset_trades(players: &mut [Bot<'_, '_>]) -> TestResult {
    for bot in players.iter_mut() {
        send(bot, wire::Intent::Cancel, wire::ResultCode::Cancelled).await?;
    }
    bots::pump(players).await?;
    Ok(())
}

async fn refusal(players: &mut [Bot<'_, '_>], world: &BundleWorld, case: &str) -> TestResult {
    reset_trades(players).await?;
    if case == "receiver-full" {
        fill_receiver(players, world).await?;
    }
    bots::approach(players, 0, 1, world, false).await?;
    let item = players[0]
        .client()?
        .inventory()
        .and_then(|v| v.entries.first())
        .copied()
        .ok_or("MP03 no refusal offer item")?;
    let before = [
        inventory(&mut players[0])?,
        inventory(&mut players[1])?,
        inventory(&mut players[2])?,
    ];
    let receipt = receipts(players).await?;
    let partner = players[1].own()?;
    match case {
        "out-of-range" => {
            // Crystal game.cpp:5230: Chebyshev distance > 2 on the same floor.
            let peer = tile(players[1].client()?.world_spatial().actor_position)?;
            let from = tile(players[0].client()?.world_spatial().actor_position)?;
            let path = route(world, from, |at| {
                at.floor == peer.floor && at.x.abs_diff(peer.x).max(at.y.abs_diff(peer.y)) == 3
            })
            .ok_or("MP03 no range-3 route")?;
            bots::walk_bot(players, 0, world, &path).await?;
            send(
                &mut players[0],
                wire::Intent::Offer {
                    partner,
                    item: item.handle.get(),
                },
                wire::ResultCode::TooFar,
            )
            .await?;
        }
        "already-trading" => {
            bots::approach(players, 2, 1, world, false).await?;
            offer(players, 0, item).await?;
            let third = players[2]
                .client()?
                .inventory()
                .and_then(|v| v.entries.first())
                .copied()
                .ok_or("MP03 C has no busy-case item")?;
            // Crystal game.cpp:5363 refuses a third player's request to a busy partner.
            send(
                &mut players[2],
                wire::Intent::Offer {
                    partner,
                    item: third.handle.get(),
                },
                wire::ResultCode::PartnerBusy,
            )
            .await?;
            // Crystal game.cpp:5359-5361 also refuses an already-trading sender
            // without cancelling its existing pair. Current server command.rs:327
            // instead closes that pair as Stale; retain that concrete reply as FAIL.
            let third_actor = players[2].own()?;
            let sender = send(
                &mut players[0],
                wire::Intent::Offer {
                    partner: third_actor,
                    item: item.handle.get(),
                },
                wire::ResultCode::PartnerBusy,
            )
            .await;
            bots::pump(players).await?;
            let unchanged = inventory(&mut players[0])? == before[0]
                && inventory(&mut players[1])? == before[1]
                && receipts(players).await? == receipt;
            if let Err(error) = sender {
                return Err(format!("busy-sender;{error};nothing_moved={unchanged};source:crates/gameplay/dispatch/src/gameplay_transport/player_trade/command.rs:327").into());
            }
            let views = [
                players[0].client()?.session_mut().player_trade_state(),
                players[1].client()?.session_mut().player_trade_state(),
            ];
            if !unchanged
                || views
                    .iter()
                    .any(|v| v.as_ref().is_none_or(|s| s.phase != 1))
            {
                return Err(format!("MP03 busy sender cancelled or moved the existing offer;reply={views:?};nothing_moved={unchanged}").into());
            }
        }
        "receiver-full" => {
            let other = players[1]
                .client()?
                .inventory()
                .and_then(|v| v.entries.first())
                .copied()
                .ok_or("MP03 full receiver has no counter item")?;
            offer(players, 0, item).await?;
            let states = offer(players, 1, other).await?;
            let trade_id = states[0]
                .trade_id
                .ok_or_else(|| format!("MP03 no ready pair;reply={states:?}"))?;
            let command = runtime::PlayerTradeCommand::new(wire::Intent::Accept { trade_id })
                .ok_or("MP03 full accept intent invalid")?;
            let first = runtime::send(players[0].client()?.session_mut(), &command).await?;
            evidence(&format!("MP03_REPLY receiver-full first-accept={first:?}"));
            match (first.status, first.result) {
                (CommandStatus::Accepted, wire::ResultCode::NoRoom) => {}
                (CommandStatus::Accepted, wire::ResultCode::Accepted) => {
                    update(&mut players[1]).await?;
                    send(&mut players[1], wire::Intent::Accept { trade_id }, wire::ResultCode::NoRoom).await?;
                }
                _ => return Err(format!("MP03 full receiver acceptance failed unexpectedly;reply={first:?};source:crates/gameplay/dispatch/src/gameplay_transport/player_trade/command.rs:504").into()),
            }
            let closed = update(&mut players[0]).await?;
            if closed.phase != 4 || closed.reason != 3 {
                return Err(format!("MP03 no-room did not close peer;reply={closed:?};source:crates/gameplay/durability/src/durability/player_trade/snapshot.rs:255").into());
            }
        }
        _ => return Err(format!("MP03 unknown case={case}").into()),
    }
    bots::pump(players).await?;
    let after = [
        inventory(&mut players[0])?,
        inventory(&mut players[1])?,
        inventory(&mut players[2])?,
    ];
    if after != before || receipts(players).await? != receipt {
        return Err(format!("MP03 refused trade changed custody;case={case};before={before:?};reply_inventory={after:?};reply_receipt={:?}", receipts(players).await?).into());
    }
    Ok(())
}

async fn fill_receiver(players: &mut [Bot<'_, '_>], world: &BundleWorld) -> TestResult {
    use oteryn_protocol_oteryn::npc_service as npc;
    // Crystal data/items/items.xml:5959-5962: starter backpack 2854 has 20 slots.
    let backpack = provisioning::item_reference(&provisioning::item_index()?, 2854)?;
    if players[1]
        .client()?
        .inventory()
        .and_then(|v| v.main_backpack)
        .is_none_or(|b| b.item_definition_ref != backpack)
    {
        return Err(format!(
            "MP03 receiver backpack differs from donor 2854;reply={:?}",
            players[1].client()?.inventory()
        )
        .into());
    }
    let window = shop(players, 1, world).await?;
    let input = native_input()?;
    let profiles: crate::content::native_gameplay::ItemProfilesDocument = serde_json::from_slice(
        &input
            .item_profiles
            .as_ref()
            .ok_or("MP03 pinned profiles absent")?
            .bytes,
    )?;
    let keys = provisioning::item_index()?;
    let nonstackable: std::collections::BTreeSet<_> = profiles
        .records
        .iter()
        .filter(|r| {
            r.admission.stack_class
                == crate::content::native_gameplay::NativeItemStackClass::NonStackable
        })
        .filter_map(|r| {
            let d = &r.production_definition;
            keys.definition_ref(&d.family, &d.production_key, &d.revision_ref)
                .map(|r| r.get())
        })
        .collect();
    let mut offers: Vec<_> = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            o.side == npc::NpcTradeSide::Buy
                && o.unit_price > 0
                && nonstackable.contains(&o.item_definition_ref)
        })
        .map(|(i, o)| (o.unit_price, i))
        .collect();
    offers.sort_unstable();
    // Try real purchases until all 20 donor-backed slots are occupied. A money,
    // catalogue or capacity prerequisite refusal is a FAIL, never a fabricated full bag.
    let mut replies = Vec::new();
    for (_, index) in offers.into_iter().take(1) {
        for _ in 0..20 {
            if players[1]
                .client()?
                .inventory()
                .is_some_and(|v| v.entries.len() == 20)
            {
                return Ok(());
            }
            let before = players[1]
                .client()?
                .inventory()
                .cloned()
                .ok_or("MP03 receiver inventory absent")?;
            let reply = shop_trade(players, 1, &window, index).await?;
            replies.push(format!("offer={:?}:reply={reply:?}", window.offers[index]));
            if reply != npc::NpcIntentDisposition::Ok {
                break;
            }
            bots::wait_observation(
                players,
                tokio::time::Instant::now() + Duration::from_secs(10),
                &format!("NPC purchase reply={reply:?};missing nonstackable inventory delta"),
                |bots, _| {
                    Ok(bots[1]
                        .session
                        .as_ref()
                        .and_then(|s| s.inventory())
                        .is_some_and(|v| v.entries.len() > before.entries.len()))
                },
            )
            .await?;
            let after = players[1]
                .client()?
                .inventory()
                .cloned()
                .ok_or("MP03 receiver inventory absent after buy")?;
            if after.entries.len() == 20 {
                return Ok(());
            }
            if after.entries.len() <= before.entries.len() {
                break;
            }
        }
    }
    Err(format!("MP03 could not establish full receiver through real purchases;reply={replies:?};inventory={:?};source:crates/gameplay/durability/src/durability/player_trade/snapshot.rs:255", players[1].client()?.inventory()).into())
}
