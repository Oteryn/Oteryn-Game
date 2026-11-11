//! G39: native modal, discovery, bed, use-at, House and bounty journeys.
//! Owners: dispatch connection/routing/{server_modal,use_item}.rs, cyclopedia.rs,
//! bed_sleep_production.rs, connection/serve/{house_hireling,house_transfer},
//! task_board_projection.rs. Each leg uses real replies plus independent state;
//! missing player prerequisites are failures, never missing-route verdicts.
use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime};
use oteryn_protocol_oteryn::{
    cyclopedia as map, server_modal as modal, task_board, world_object as object,
};
use sqlx::Connection;

const LEGS: &[&str] = &[
    "modal",
    "discovery-poi",
    "bed-sleep",
    "use-at",
    "house-hireling",
    "house-transfer",
    "task-reroll",
];

fn token(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join("-")
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    // Keep ownership outside the cancelled future so timeout/error paths still log out.
    let mut active = None;
    let result =
        tokio::time::timeout(Duration::from_secs(600), journey(clients, &mut active)).await;
    let cleanup = match active.as_mut() {
        Some(client) => logout(client).await,
        None => Ok(()),
    };
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "all-seven-legs-with-state-and-relog".into()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "needed:journey-completes-within-600s".into()),
    };
    let (status, detail) = match cleanup {
        Ok(()) => (status, detail),
        Err(error) => (
            "FAIL",
            format!("{detail};logout:{}", token(&error.to_string())),
        ),
    };
    evidence(&format!(
        "e2e_all scenario=G39.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn command(
    client: &mut DevClientSession,
    capability: u32,
    kind: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(capability, kind, payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("needed:accepted-command:{kind};observed:{:?}", reply.status).into());
    }
    Ok(reply.payload)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if tokio::time::timeout(Duration::from_secs(10), client.session_mut().logout())
        .await
        .map_err(|_| "needed:logout-completes-within-10s")??
        .result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("needed:accepted-final-logout".into());
    }
    Ok(())
}

async fn map_command(client: &mut DevClientSession, operation: u8) -> TestResult<map::MapResult> {
    let (kind, payload) = client_runtime::cyclopedia_map::request(operation, 0)
        .map_err(|e| format!("map-encode:{e:?}"))?;
    map::decode_result(&command(client, map::CAPABILITY_CYCLOPEDIA_MAP_V1, kind, &payload).await?)
        .map_err(|e| format!("map-decode:{e:?}").into())
}

async fn discovery(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<map::MapResult> {
    let before = map_command(client, 0).await?;
    let started = map_command(client, 1).await?;
    if started.code != 0 {
        return Err(format!("needed:undiscovered-premium-map-area;observed:{started:?}").into());
    }
    let here = tile(client.world_spatial().actor_position)?;
    let path = started
        .visible
        .iter()
        .filter_map(|&(x, y, floor)| {
            route(world, here, |at| {
                at.x == x && at.y == y && at.floor == floor
            })
        })
        .min_by_key(Vec::len)
        .ok_or("needed:visible-player-reachable-poi")?;
    walk(client, world, &path).await?;
    let _ = map_command(client, 3).await?;
    let after = map_command(client, 0).await?;
    if after.found <= started.found || after.revision <= before.revision {
        return Err(format!(
            "needed:poi-found-and-map-revision-delta;before:{before:?};after:{after:?}"
        )
        .into());
    }
    Ok(after)
}

async fn board(client: &mut DevClientSession, action: u8) -> TestResult<task_board::full::Reply> {
    Ok(client_runtime::task_board::send_full(
        client.session_mut(),
        &task_board::full::Request {
            action,
            selection: 0,
            race: String::new(),
        },
    )
    .await?)
}

async fn earn_reroll(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use crate::map::view::MapFacts;
    use client_runtime::daily_reward::{self as runtime, DailyRewardCommand, DailyRewardReply};
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let content = crate::content::daily_reward::DailyRewardContent::catalogue()
        .ok_or("needed:daily-reward-content")?;
    let index = provisioning::item_index()?;
    let references = content
        .shrine_source_ids
        .iter()
        .map(|id| provisioning::item_reference(&index, *id))
        .collect::<TestResult<Vec<_>>>()?;
    let start = tile(client.world_spatial().actor_position)?;
    let facts = world.facts().pinned();
    let path = world.base().tiles().filter(|(_, x, y, _)| start.x.abs_diff(*x) <= 160 && start.y.abs_diff(*y) <= 160).find_map(|(floor, x, y, tile_data)| {
        let at = TilePos { floor, x, y };
        let shrine = tile_data.ids().iter().zip(tile_data.depths()).filter(|(_, depth)| **depth == 0).enumerate().any(|(ordinal, (id, _))| u8::try_from(ordinal).ok().and_then(|n| facts.base_entry(at, n, *id)).is_some_and(|entry| matches!(entry.definition, MapDefinition::Item(reference) if references.contains(&reference))));
        if !shrine { return None; }
        route(world, start, |candidate| candidate.floor == floor && candidate.x.abs_diff(x).max(candidate.y.abs_diff(y)) <= 1)
    }).ok_or("needed:reachable-daily-reward-shrine-for-reroll-token")?;
    walk(client, world, &path).await?;
    let wall = runtime::send(client.session_mut(), &DailyRewardCommand::Query).await?;
    if wall.status != CommandStatus::Accepted
        || !matches!(wall.reply, Some(DailyRewardReply::Wall(wall)) if wall.claim_enabled && !wall.claimed)
    {
        return Err(format!("needed:unclaimed-daily-reward-for-token;observed:{wall:?}").into());
    }
    let claim = runtime::send(client.session_mut(), &DailyRewardCommand::Claim).await?;
    if claim.status != CommandStatus::Accepted
        || !matches!(claim.reply, Some(DailyRewardReply::Result(result)) if result.disposition == 1 && result.transaction_id.is_some())
    {
        return Err(format!("needed:daily-reward-claim-for-token;observed:{claim:?}").into());
    }
    Ok(())
}

async fn reroll(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<task_board::full::Reply> {
    if board(client, 4).await?.bounty.rerolls == 0 {
        earn_reroll(client, world).await?;
    }
    let opened = board(client, 7).await?;
    if opened.refusal != 0 {
        return Err(format!("needed:bounty-offer-admission;refusal:{}", opened.refusal).into());
    }
    let before = board(client, 4).await?;
    let changed = board(client, 11).await?;
    if changed.refusal != 0 {
        return Err(format!(
            "needed:earned-reroll-token-and-offered-bounty;tokens:{};refusal:{}",
            before.bounty.rerolls, changed.refusal
        )
        .into());
    }
    let after = board(client, 4).await?;
    if before.bounty.rerolls.checked_sub(1) != Some(after.bounty.rerolls)
        || after.bounty.daily_reroll_at != 0
        || after.bounty.offers.is_empty()
        || changed.bounty != after.bounty
    {
        return Err("needed:one-token-debit-and-new-offers-without-free-reroll".into());
    }
    Ok(after)
}

async fn owned_house(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let key: Option<String> = sqlx::query_scalar("SELECT house_key FROM game_house_ownership WHERE owner_character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid ORDER BY house_key LIMIT 1")
        .bind(clients.characters[1].as_slice()).bind(clients.world.as_slice()).fetch_optional(&mut db).await?;
    db.close().await?;
    key.ok_or_else(|| "needed:player-acquired-house-with-entry-access".into())
}

async fn transfer(
    client: &mut DevClientSession,
    house: &str,
    exit: bool,
) -> TestResult<client_runtime::house_transfer::HouseTransferAck> {
    use client_runtime::house_transfer as wire;
    let payload = wire::encode_house_transfer_intent(&wire::HouseTransferIntent {
        exit,
        house: house.into(),
    })
    .map_err(|e| format!("transfer-encode:{e:?}"))?;
    let bytes = command(
        client,
        wire::CAPABILITY_HOUSE_TRANSFER_V1,
        wire::COMMAND_TYPE_HOUSE_TRANSFER,
        &payload,
    )
    .await?;
    let ack = wire::decode_house_transfer_ack(&bytes)
        .map_err(|e| format!("needed:house-entry-at-player-position;reply:{e:?}"))?;
    if ack.house.as_deref() != if exit { None } else { Some(house) } {
        return Err("needed:matching-transfer-scope-ack".into());
    }
    // The scope snapshot is a server frame independent of the command acknowledgement.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    loop {
        let at = client.world_spatial().actor_position;
        if exit && [at.x, at.y, i32::from(at.floor)] == ack.position {
            break;
        }
        if !exit
            && client
                .session_mut()
                .gated_snapshot(wire::STATE_DOMAIN_HOUSE_SCOPE)
                .is_some_and(|snapshot| {
                    wire::decode_house_transfer_ack(&snapshot.payload).is_ok_and(|scope| {
                        scope.transfer == ack.transfer
                            && scope.scope_generation == ack.scope_generation
                            && scope.house == ack.house
                    })
                })
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("needed:scope-snapshot-for-committed-transfer".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(ack)
}

async fn hireling(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    use client_runtime::hireling as runtime;
    use runtime::hireling as wire;
    let house = owned_house(clients).await?;
    transfer(client, &house, false).await?;
    let at = tile(client.world_spatial().actor_position)?;
    let prior: std::collections::BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| entry.kind == oteryn_dev_client::EntityKind::Npc)
        .map(|entry| entry.entity.identity)
        .collect();
    let response = runtime::send(
        client.session_mut(),
        &runtime::HirelingCommand(wire::Request::Summon {
            x: at.x,
            y: at.y,
            floor: at.floor,
        }),
    )
    .await?;
    if response.status != CommandStatus::Accepted || response.active != Some(true) {
        return Err(
            format!("needed:owned-hireling-and-free-house-tile;observed:{response:?}").into(),
        );
    }
    let query = runtime::send(
        client.session_mut(),
        &runtime::HirelingCommand(wire::Request::Query),
    )
    .await?;
    if query.status != CommandStatus::Accepted || query.active != Some(true) {
        return Err("needed:active-hireling-state-after-summon".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    while !client.world_entities().is_some_and(|entities| {
        entities.iter().any(|entry| {
            entry.kind == oteryn_dev_client::EntityKind::Npc
                && !prior.contains(&entry.entity.identity)
        })
    }) {
        if tokio::time::Instant::now() >= deadline {
            return Err("needed:summoned-hireling-entity-frame".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let dismissed = runtime::send(
        client.session_mut(),
        &runtime::HirelingCommand(wire::Request::Dismiss),
    )
    .await?;
    if dismissed.status != CommandStatus::Accepted || dismissed.active != Some(false) {
        return Err("needed:dismissed-hireling-state".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    while client.world_entities().is_some_and(|entities| {
        entities.iter().any(|entry| {
            entry.kind == oteryn_dev_client::EntityKind::Npc
                && !prior.contains(&entry.entity.identity)
        })
    }) {
        if tokio::time::Instant::now() >= deadline {
            return Err("needed:dismissed-hireling-entity-removal".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    transfer(client, &house, true).await?;
    Ok(())
}

// Use only a bed the server actually projected beside the player, never a fabricated handle.
fn bed_target(client: &mut DevClientSession) -> TestResult<object::UseTarget> {
    use oteryn_protocol_oteryn::world_map as wire;
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("needed:house-map-snapshot")?;
    let map = wire::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|e| format!("map-snapshot:{e:?}"))?;
    let index = provisioning::item_index()?;
    let at = client.world_spatial().actor_position;
    for tile in &map.tiles {
        if tile.position.floor != at.floor
            || tile
                .position
                .x
                .abs_diff(at.x)
                .max(tile.position.y.abs_diff(at.y))
                > 1
        {
            continue;
        }
        for item in &tile.items {
            let wire::MapDefinition::Item(reference) = item.definition else {
                continue;
            };
            let Some((key, _)) = index.definition_at(reference) else {
                continue;
            };
            // Source bed halves in the native house catalogue.
            if !(1754..=1761).any(|id| key == format!("oteryn:item.tibia.i{id}")) {
                continue;
            }
            match item.origin {
                wire::MapOrigin::Handle(handle) => return Ok(object::UseTarget::Item(handle)),
                wire::MapOrigin::BaseOrdinal {
                    ordinal,
                    object_revision,
                } => {
                    let key = crate::map::view::placement_key(super::tile(tile.position)?, ordinal)
                        .ok_or("bed-placement")?;
                    let mut placement = map.header.bundle_digest.to_vec();
                    placement.extend(key.to_be_bytes());
                    return Ok(object::UseTarget::WorldObject(object::WorldObjectTarget {
                        placement,
                        expected_revision: object_revision,
                    }));
                }
                wire::MapOrigin::DisplayOnly => {}
            }
        }
    }
    Err("needed:adjacent-authorized-house-bed".into())
}

async fn modal_query(client: &mut DevClientSession) -> TestResult<modal::Reply> {
    modal::Reply::decode(
        &command(
            client,
            modal::CAPABILITY_SERVER_MODAL_V1,
            modal::COMMAND_TYPE_SERVER_MODAL_QUERY,
            modal::QUERY_WITH_TERMINAL_REPLY,
        )
        .await?,
    )
    .map_err(Into::into)
}

async fn acknowledge(client: &mut DevClientSession, dialog: &modal::Dialog) -> TestResult {
    let answer = modal::Answer {
        dialog_id: dialog.id,
        button_id: dialog
            .cancel_button
            .or_else(|| dialog.buttons.first().map(|button| button.0))
            .ok_or("needed:modal-button")?,
        choice_id: None,
    };
    let bytes = command(
        client,
        modal::CAPABILITY_SERVER_MODAL_V1,
        modal::COMMAND_TYPE_SERVER_MODAL_ANSWER,
        &answer.encode()?,
    )
    .await?;
    if modal::Reply::decode(&bytes)? != modal::Reply::Answered(answer)
        || modal_query(client).await? != modal::Reply::None
    {
        return Err("needed:modal-answer-and-pending-dialog-removal".into());
    }
    Ok(())
}

async fn modal_notice(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    if let modal::Reply::Dialog(dialog) = modal_query(client).await? {
        return acknowledge(client, &dialog).await;
    }
    // The bed is a player-triggered modal producer even without an operator notice.
    bed_modal(client, clients, false, &mut false).await?;
    Ok(())
}

async fn bed_modal(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    sleep: bool,
    terminal: &mut bool,
) -> TestResult<Option<[u8; 16]>> {
    let house = owned_house(clients).await?;
    transfer(client, &house, false).await?;
    if let modal::Reply::Dialog(dialog) = modal_query(client).await? {
        acknowledge(client, &dialog).await?;
    }
    let target = bed_target(client)?;
    let payload = object::encode_item_use_intent(&object::UseIntent {
        target,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|e| format!("bed-use:{e:?}"))?;
    let reply = command(
        client,
        object::CAPABILITY_ITEM_USE_V1,
        object::COMMAND_TYPE_USE_INTENT,
        &payload,
    )
    .await?;
    let used = object::decode_use_result(&reply).map_err(|e| format!("bed-reply:{e:?}"))?;
    if used != object::UseDisposition::BedChooseSkill {
        return Err(format!("needed:premium-authorized-unoccupied-bed;observed:{used:?}").into());
    }
    let modal::Reply::Dialog(dialog) = modal_query(client).await? else {
        return Err("needed:server-skill-dialog-after-bed-use".into());
    };
    let answer = if sleep {
        modal::Answer {
            dialog_id: dialog.id,
            button_id: dialog
                .buttons
                .iter()
                .find(|(id, _)| Some(*id) != dialog.cancel_button)
                .ok_or("needed:skill-confirm-button")?
                .0,
            choice_id: Some(dialog.choices.first().ok_or("needed:skill-choice")?.0),
        }
    } else {
        modal::Answer {
            dialog_id: dialog.id,
            button_id: dialog.cancel_button.ok_or("needed:modal-cancel-button")?,
            choice_id: None,
        }
    };
    let payload = answer.encode()?;
    let reply = modal::Reply::decode(
        &command(
            client,
            modal::CAPABILITY_SERVER_MODAL_V1,
            modal::COMMAND_TYPE_SERVER_MODAL_ANSWER,
            &payload,
        )
        .await?,
    )?;
    if sleep {
        if reply != modal::Reply::AnsweredAndLoggedOut(answer) {
            return Err(format!("needed:terminal-sleep-ack;observed:{reply:?}").into());
        }
        *terminal = true;
        Ok(Some(*client.join_snapshot().game_session_id.as_bytes()))
    } else {
        if reply != modal::Reply::Answered(answer)
            || modal_query(client).await? != modal::Reply::None
        {
            return Err("needed:modal-answer-and-dialog-removal".into());
        }
        transfer(client, &house, true).await?;
        Ok(None)
    }
}

async fn use_at(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
    // A real rune must be acquired by this character; an arbitrary item or missing-source
    // refusal cannot demonstrate the new cursor-position semantics.
    let index = provisioning::item_index()?;
    let rune = client
        .inventory()
        .and_then(|inventory| {
            inventory.entries.iter().find(|item| {
                index
                    .definition_at(item.item_definition_ref)
                    .is_some_and(|(key, _)| key == "oteryn:item.tibia.i3191")
            })
        })
        .copied()
        .ok_or("needed:player-acquired-great-fireball-rune:3191")?;
    let before = client.inventory().cloned().ok_or("needed:rune-inventory")?;
    client.take_events();
    let at = client.world_spatial().actor_position;
    let payload = object::encode_item_use_intent(&object::UseIntent {
        target: object::UseTarget::Item(rune.handle),
        use_with: None,
        use_with_map_item: None,
        use_with_position: Some(SpellTargetPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        }),
    })
    .map_err(|e| format!("use-at-encode:{e:?}"))?;
    let reply = command(
        client,
        object::CAPABILITY_ITEM_USE_V1,
        object::COMMAND_TYPE_USE_INTENT,
        &payload,
    )
    .await?;
    let used = object::decode_use_result(&reply).map_err(|e| format!("use-at-decode:{e:?}"))?;
    if used != object::UseDisposition::Committed {
        return Err(format!("needed:eligible-rune-target-and-skill;observed:{used:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    let mut effect = false;
    loop {
        effect |= client
            .take_events()
            .iter()
            .any(|event| rune_effect(event, at));
        let remaining = client
            .inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|item| item.handle == rune.handle)
            })
            .map_or(0, |item| item.count.get());
        if effect && rune.count.get().checked_sub(1) == Some(remaining) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("needed:rune-debit-and-own-effect-at-requested-tile".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    if client.inventory() == Some(&before) {
        return Err("needed:server-rune-consumption-delta".into());
    }
    Ok(())
}

fn rune_effect(event: &oteryn_dev_client::SessionEvent, target: ActorPosition) -> bool {
    use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationSource};
    let oteryn_dev_client::SessionEvent::Presentation(delta) = event else {
        return false;
    };
    delta.value.iter().any(|effect| {
        matches!(effect.kind,
        PresentationEventKind::MagicEffect { position, source: PresentationSource::Own, .. }
            if position == target)
            && delta.value.iter().any(|missile| {
                missile.decision_ordinal == effect.decision_ordinal
                    && matches!(missile.kind, PresentationEventKind::Missile {
                to, source: PresentationSource::Own, ..
            } if to == target)
            })
    })
}

async fn sleep_receipt(clients: &SeamClients<'_>, session: [u8; 16]) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_bed_sleep_receipts WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_slice()).fetch_one(&mut db).await?;
    db.close().await?;
    if count != 1 {
        return Err("needed:one-durable-bed-sleep-receipt".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, active: &mut Option<DevClientSession>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut failures = Vec::new();
    for &leg in LEGS {
        let client = active.insert(connect_world(clients, 0xf5).await?);
        let mut terminal = false;
        let mut map_after = None;
        let mut board_after = None;
        let mut slept = None;
        let mut transfer_after = None;
        let mut rune_after = None;
        let result: TestResult = match tokio::time::timeout(Duration::from_secs(70), async {
            match leg {
                "modal" => {
                    modal_notice(client, clients).await?;
                }
                "discovery-poi" => {
                    map_after = Some(discovery(client, world).await?);
                }
                "bed-sleep" => {
                    slept = bed_modal(client, clients, true, &mut terminal).await?;
                }
                "use-at" => {
                    use_at(client).await?;
                    rune_after = client.inventory().map(|inventory| {
                        inventory
                            .entries
                            .iter()
                            .map(|item| (item.item_definition_ref, item.count, item.sub_type))
                            .collect::<Vec<_>>()
                    });
                }
                "house-hireling" => {
                    hireling(client, clients).await?;
                }
                "house-transfer" => {
                    let house = owned_house(clients).await?;
                    let entered = transfer(client, &house, false).await?;
                    let exited = transfer(client, &house, true).await?;
                    transfer_after = Some(exited.clone());
                    if exited.scope_generation <= entered.scope_generation {
                        return Err("needed:advanced-house-scope-generation".into());
                    }
                }
                "task-reroll" => {
                    board_after = Some(reroll(client, world).await?);
                }
                _ => return Err("unknown-leg".into()),
            }
            Ok(())
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err("needed:leg-completes-within-70s".into()),
        };
        let ended = if terminal {
            Ok(())
        } else {
            logout(client).await
        };
        active.take();
        let result: TestResult = async {
            ended?;
            result?;
            if let Some(session) = slept {
                sleep_receipt(clients, session).await?;
            }
            let fresh = active.insert(connect_world(clients, 0xf6).await?);
            let checked: TestResult = async {
                if leg == "modal" && modal_query(fresh).await? != modal::Reply::None {
                    return Err("needed:relog-answered-modal-remains-closed".into());
                }
                if let Some(before) = rune_after {
                    let after = fresh.inventory().ok_or("needed:relog-rune-inventory")?;
                    let entries: Vec<_> = after
                        .entries
                        .iter()
                        .map(|item| (item.item_definition_ref, item.count, item.sub_type))
                        .collect();
                    if entries != before {
                        return Err("needed:relog-rune-consumption".into());
                    }
                }
                if let Some(before) = map_after {
                    let after = map_command(fresh, 0).await?;
                    if after.found != before.found || after.revision < before.revision {
                        return Err("needed:relog-poi-discovery-state".into());
                    }
                }
                if let Some(before) = board_after {
                    let after = board(fresh, 4).await?;
                    if before.bounty.rerolls != after.bounty.rerolls
                        || before.bounty.offers != after.bounty.offers
                    {
                        return Err("needed:relog-reroll-balance-and-offers".into());
                    }
                }
                if let Some(session) = slept {
                    sleep_receipt(clients, session).await?;
                }
                if let Some(exited) = transfer_after {
                    let at = fresh.world_spatial().actor_position;
                    if [at.x, at.y, i32::from(at.floor)] != exited.position {
                        return Err("needed:relog-house-exit-position".into());
                    }
                }
                if leg == "house-hireling" {
                    use client_runtime::hireling as runtime;
                    let house = owned_house(clients).await?;
                    transfer(fresh, &house, false).await?;
                    let reply = runtime::send(
                        fresh.session_mut(),
                        &runtime::HirelingCommand(runtime::hireling::Request::Query),
                    )
                    .await?;
                    if reply.status != CommandStatus::Accepted || reply.active != Some(false) {
                        return Err("needed:relog-dismissed-hireling-state".into());
                    }
                    transfer(fresh, &house, true).await?;
                }
                Ok(())
            }
            .await;
            logout(fresh).await?;
            active.take();
            checked
        }
        .await;
        match result {
            Ok(()) => evidence(&format!(
                "G39_CHECK leg={leg} status=PASS evidence=command-state-relog"
            )),
            Err(error) => {
                let detail = token(&error.to_string());
                evidence(&format!(
                    "G39_CHECK leg={leg} status=FAIL evidence={detail}"
                ));
                failures.push(format!("{leg}:{detail}"));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join(";").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_evidence_is_one_token() {
        assert_eq!(
            token("needed:house entry\nrefused"),
            "needed:house-entry-refused"
        );
        assert_eq!(LEGS.len(), 7);
    }
    #[test]
    fn rune_effect_requires_own_missile_and_effect_in_the_same_decision() {
        use oteryn_dev_client::{AppliedDelta, SessionEvent};
        use oteryn_protocol_oteryn::presentation::{
            PresentationEvent, PresentationEventKind, PresentationSource,
        };
        let at = ActorPosition {
            x: 100,
            y: 101,
            floor: 7,
        };
        let missile = PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::Missile {
                from: at,
                to: at,
                missile_id: 1,
                source: PresentationSource::Own,
            },
        };
        let effect = PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 2,
            kind: PresentationEventKind::MagicEffect {
                position: at,
                effect_id: 1,
                source: PresentationSource::Own,
            },
        };
        let mut event = SessionEvent::Presentation(AppliedDelta {
            server_sequence: 1,
            base_revision: 1,
            new_revision: 2,
            value: vec![missile, effect],
        });
        assert!(rune_effect(&event, at));
        assert!(!rune_effect(&event, ActorPosition { x: 102, ..at }));
        if let SessionEvent::Presentation(delta) = &mut event {
            delta.value[1].decision_ordinal = 2;
        }
        assert!(!rune_effect(&event, at));
        if let SessionEvent::Presentation(delta) = &mut event {
            delta.value[1] = PresentationEvent {
                kind: PresentationEventKind::MagicEffect {
                    position: at,
                    effect_id: 1,
                    source: PresentationSource::Others,
                },
                ..effect
            };
        }
        assert!(!rune_effect(&event, at));
    }
}
