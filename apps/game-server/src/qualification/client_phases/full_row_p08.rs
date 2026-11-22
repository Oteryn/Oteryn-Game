//! P08 full-row native-client journey.
//!
//! Scenario derived from the P08 matrix/audit row and its production dispatch owner:
//! 1. Browse the admitted proficiency catalogue and current tracks; expect bounded native reply
//!    pages whose definition, XP, level, selections, thresholds and perk descriptions agree.
//! 2. Walk to a compiled-world rat, select it with the ordinary attack command and finish the
//!    fight; expect the server to credit weapon proficiency XP only for an equipped bound weapon.
//! 3. Select an unlocked perk; expect an accepted selection reply and a matching browse delta.
//! 4. Exercise modify, rank-up, orb-rank, reshape-offer, reshape-choose and clear with the
//!    committed revision; expect typed replies and the corresponding slot transitions/costs.
//! 5. Send a stale-revision modification as the refusal path; expect `StaleRevision` and no
//!    change to the track page.
//! 6. Log out and obtain a fresh admission; expect XP, selections and shaped slots/catalyst
//!    consumption to remain durable. A server/content refusal is reported with the exact step
//!    and observed typed reply, but only after all mechanic commands have actually been sent.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::weapon_proficiency as wire;
use std::num::NonZeroU32;

#[derive(Default)]
struct Observations {
    failures: Vec<String>,
}

impl Observations {
    fn require(&mut self, step: &str, condition: bool, observed: impl std::fmt::Debug) {
        if !condition {
            self.failures.push(format!("step-{step}:{observed:?}"));
        }
    }
}

async fn command(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_WEAPON_PROFICIENCY_V1, kind, payload)
        .await?)
}

async fn browse(client: &mut DevClientSession, offset: u32) -> TestResult<wire::ProficiencyPage> {
    let reply = command(
        client,
        wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
        &offset.to_be_bytes(),
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-browse:status={:?}", reply.status).into());
    }
    wire::ProficiencyPage::decode(&reply.payload)
        .ok_or_else(|| "step-browse:malformed proficiency page".into())
}

async fn catalogue(
    client: &mut DevClientSession,
) -> TestResult<Vec<wire::ProficiencyCatalogueEntry>> {
    let mut entries = Vec::new();
    loop {
        let offset = u32::try_from(entries.len())?;
        let reply = command(
            client,
            wire::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
            &offset.to_be_bytes(),
        )
        .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("step=catalogue offset={offset} status={:?}", reply.status).into());
        }
        let page = wire::ProficiencyCataloguePage::decode(&reply.payload)
            .ok_or("step=catalogue malformed page")?;
        if page.offset != offset {
            return Err(format!(
                "step=catalogue requested offset={offset} returned={}",
                page.offset
            )
            .into());
        }
        let empty = page.entries.is_empty();
        entries.extend(page.entries);
        if entries.len() >= usize::try_from(page.total)? {
            return Ok(entries);
        }
        if empty {
            return Err(
                format!("step=catalogue empty intermediate page at offset={offset}").into(),
            );
        }
    }
}

async fn browse_weapon(
    client: &mut DevClientSession,
    definition: NonZeroU32,
) -> TestResult<wire::ProficiencyPage> {
    let mut offset = 0;
    loop {
        let page = browse(client, offset).await?;
        if page.offset != offset || page.rows.is_empty() {
            return Err(format!(
                "step=browse definition={definition} invalid or empty page at offset={offset}"
            )
            .into());
        }
        if page.rows.iter().any(|row| row.definition == definition) {
            return Ok(page);
        }
        offset = offset
            .checked_add(u32::try_from(page.rows.len())?)
            .ok_or("step=browse offset overflow")?;
        if offset >= page.total {
            return Err(format!(
                "step=browse definition={definition} missing from the admitted tracks"
            )
            .into());
        }
    }
}

async fn select(
    client: &mut DevClientSession,
    definition: NonZeroU32,
) -> TestResult<wire::SelectPerkReply> {
    let payload = wire::SelectPerkRequest {
        definition,
        level: 0,
        perk: Some(0),
    }
    .encode();
    let reply = command(client, wire::COMMAND_TYPE_PROFICIENCY_SELECT_PERK, &payload).await?;
    wire::SelectPerkReply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step-select:status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into()
    })
}

async fn modify(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    operation: wire::ModificationOperation,
    revision: u64,
) -> TestResult<wire::ModificationReply> {
    let request = wire::ModificationRequest {
        definition,
        slot: 1,
        operation,
        expected_revision: revision,
    };
    let reply = command(client, operation.command_type(), &request.encode()).await?;
    wire::ModificationReply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step-modify-{operation:?}:status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into()
    })
}

async fn acquire_weapon(
    client: &mut DevClientSession,
    world: &BundleWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    definitions: &[wire::ProficiencyCatalogueEntry],
) -> TestResult<NonZeroU32> {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    let (definition, request) = weapon_quote(client, world, &metadata.npcs, definitions).await?;
    let reply = npc_shop::buy(client.session_mut(), &request).await?;
    if reply == NpcIntentDisposition::Ok {
        return Ok(definition);
    }
    if reply != NpcIntentDisposition::Rejected {
        return Err(format!(
            "step=weapon-shop buy definition={definition} price={} reply={reply:?}",
            request.expected_unit_price
        )
        .into());
    }
    let needed = u64::from(request.expected_unit_price);
    let items = provisioning::item_index()?;
    let coins = [
        (provisioning::item_reference(&items, 3031)?, 1),
        (provisioning::item_reference(&items, 3035)?, 100),
        (provisioning::item_reference(&items, 3043)?, 10_000),
    ];
    let carried = |client: &DevClientSession| -> TestResult<u64> {
        Ok(client
            .inventory()
            .ok_or("step=weapon-funding inventory absent")?
            .entries
            .iter()
            .filter_map(|entry| {
                coins
                    .iter()
                    .find(|(reference, _)| *reference == entry.item_definition_ref)
                    .map(|(_, value)| u64::from(entry.count.get()) * value)
            })
            .sum())
    };
    let before = carried(client)?;
    for _ in 0..12 {
        if carried(client)? >= needed {
            break;
        }
        travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client).await?;
    }
    let funded = carried(client)?;
    if funded < needed || funded == before {
        return Err(format!("step=weapon-shop buy definition={definition} price={needed} reply={reply:?} carried-gold={before}->{funded}").into());
    }
    let (definition, request) = weapon_quote(client, world, &metadata.npcs, definitions).await?;
    let reply = npc_shop::buy(client.session_mut(), &request).await?;
    if reply != NpcIntentDisposition::Ok {
        return Err(format!(
            "step=weapon-shop funded buy definition={definition} price={} reply={reply:?}",
            request.expected_unit_price
        )
        .into());
    }
    Ok(definition)
}

async fn weapon_quote(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    definitions: &[wire::ProficiencyCatalogueEntry],
) -> TestResult<(
    NonZeroU32,
    oteryn_protocol_oteryn::npc_service::NpcTradeIntent,
)> {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as npc;
    let sam = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.sam")
        .ok_or("step=weapon-shop authored Sam placement missing")?;
    let from = tile(client.world_spatial().actor_position)?;
    let near = |at: TilePos| {
        sam.placements
            .iter()
            .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 3)
    };
    let path = route(world, from, near).ok_or("step=weapon-shop Sam has no walking route")?;
    walk(client, world, &path).await?;
    let basis = spatial_basis::current(client)?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && sam.placements.iter().any(|p| {
                        entry.position.x == i32::from(p.x)
                            && entry.position.y == i32::from(p.y)
                            && entry.position.floor == basis.floor(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("step=weapon-shop Sam actor absent at placement")?;
    for text in ["hi", "trade"] {
        let reply = npc_shop::talk(
            client.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("step=weapon-shop talk={text} reply={reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("step=weapon-shop decode={error:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy
                && definitions
                    .iter()
                    .any(|entry| entry.definition == offer.item_definition_ref)
        })
        .min_by_key(|(_, offer)| offer.unit_price)
        .ok_or("step=weapon-shop no live offer matching an admitted proficiency track")?;
    let definition =
        NonZeroU32::new(offer.item_definition_ref).ok_or("step=weapon-shop zero definition")?;
    Ok((
        definition,
        npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("step=weapon-shop closed window")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    ))
}

async fn equip_weapon(client: &mut DevClientSession, definition: NonZeroU32) -> TestResult {
    use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};
    let equipped = |client: &DevClientSession| {
        client.inventory().is_some_and(|inventory| {
            inventory.equipment.iter().any(|entry| {
                entry.slot == EquipmentSlot::RightHand
                    && entry.item.item_definition_ref == definition
            })
        })
    };
    if equipped(client) {
        return Ok(());
    }
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == definition)
        })
        .map(|entry| entry.handle)
        .ok_or("step=weapon-equip purchased item missing from backpack")?;
    let moved = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source,
            destination: oteryn_dev_client::ItemMoveDestination::Equipment(
                EquipmentSlot::RightHand,
            ),
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved || !equipped(client) {
        return Err(format!("step=weapon-equip definition={definition} reply={moved:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, observed: &mut Observations) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xa2).await?;
    let initial_session = client.join_snapshot().game_session_id;
    let catalogue = catalogue(&mut client).await?;
    let definition = if let Some(definition) = client.inventory().and_then(|inventory| {
        inventory
            .equipment
            .iter()
            .map(|entry| &entry.item)
            .chain(&inventory.entries)
            .map(|item| item.item_definition_ref)
            .find(|definition| {
                catalogue
                    .iter()
                    .any(|entry| entry.definition == definition.get())
            })
    }) {
        definition
    } else {
        acquire_weapon(&mut client, world, &metadata, &catalogue).await?
    };
    equip_weapon(&mut client, definition).await?;
    let before = browse_weapon(&mut client, definition).await?;
    let browse_offset = before.offset;
    let catalogue_entry = catalogue
        .iter()
        .find(|entry| entry.definition == definition.get())
        .ok_or("step-1:chosen definition missing from catalogue")?;
    let browse_row = before
        .rows
        .iter()
        .find(|row| row.definition == definition)
        .ok_or("step-1:chosen definition missing from browse page")?;
    let catalogue_level = u8::try_from(
        catalogue_entry
            .thresholds
            .iter()
            .take_while(|threshold| catalogue_entry.experience >= **threshold)
            .count(),
    )?;
    observed.require(
        "1-catalogue-browse-agreement",
        browse_row.name == catalogue_entry.name
            && browse_row.experience == catalogue_entry.experience
            && browse_row.level == catalogue_level
            && browse_row.selections
                == catalogue_entry
                    .levels
                    .iter()
                    .map(|level| level.selected)
                    .collect::<Vec<_>>(),
        (catalogue_entry, browse_row),
    );

    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    client_hunt::hunt(&mut client).await?;
    let mut after_kill = browse(&mut client, browse_offset).await?;
    observed.require(
        "2-xp-delta",
        after_kill.rows.iter().any(|after| {
            after.definition == definition && after.experience > browse_row.experience
        }),
        (&before.rows, &after_kill.rows),
    );

    // The first selection is only legal after the equipped track earns level one.
    let first_threshold = catalogue_entry.thresholds[0];
    for _ in 0..16 {
        let xp = after_kill
            .rows
            .iter()
            .find(|row| row.definition == definition)
            .ok_or("step=perk-unlock equipped definition absent from browse")?
            .experience;
        if xp >= first_threshold {
            break;
        }
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(&mut client).await?;
        after_kill = browse(&mut client, browse_offset).await?;
        if !after_kill
            .rows
            .iter()
            .any(|row| row.definition == definition && row.experience > xp)
        {
            return Err(format!(
                "step=perk-unlock definition={definition} stalled-xp={xp} need={first_threshold}"
            )
            .into());
        }
    }
    if !after_kill
        .rows
        .iter()
        .any(|row| row.definition == definition && row.experience >= first_threshold)
    {
        return Err(format!("step=perk-unlock need={first_threshold} player-earned proficiency XP on definition={definition}").into());
    }

    let selected = select(&mut client, definition).await?;
    observed.require(
        "select",
        selected.result == wire::SelectPerkResult::Accepted,
        &selected,
    );
    let after_select = browse(&mut client, browse_offset).await?;
    let selected_row = after_select
        .rows
        .iter()
        .find(|row| row.definition == definition);
    observed.require(
        "3-selection-browse-delta",
        selected.result == wire::SelectPerkResult::Accepted
            && selected.selections.first() == Some(&Some(0))
            && selected_row.is_some_and(|row| {
                row.selections == selected.selections
                    && after_kill
                        .rows
                        .iter()
                        .find(|before| before.definition == definition)
                        != Some(row)
            }),
        (&after_kill, &selected, &after_select),
    );
    let operations = [
        wire::ModificationOperation::Modify { level: 0 },
        wire::ModificationOperation::RankUp,
        wire::ModificationOperation::OrbRank,
        wire::ModificationOperation::ReshapeOffer,
        wire::ModificationOperation::ReshapeChoose { choice: Some(0) },
        wire::ModificationOperation::Clear,
    ];
    let mut previous_slot: Option<wire::ModifiedSlot> = None;
    for (index, operation) in operations.into_iter().enumerate() {
        let reply = modify(&mut client, definition, operation, index as u64 + 1).await?;
        observed.require(
            "modification",
            reply.result == wire::ModificationResult::Accepted,
            (operation, reply),
        );
        let transition_matches = match (operation, previous_slot, reply.slot) {
            (wire::ModificationOperation::Modify { level: 0 }, None, Some(after)) => {
                after.level == 0 && after.rank == 0 && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::RankUp, Some(before), Some(after)) => {
                after.level == before.level
                    && after.entry_index == before.entry_index
                    && after.rank == before.rank + 1
                    && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::OrbRank, Some(before), Some(after)) => {
                after.level == before.level
                    && after.entry_index == before.entry_index
                    && after.rank == 10
                    && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::ReshapeOffer, Some(before), Some(after)) => {
                after.level == before.level
                    && after.entry_index == before.entry_index
                    && after.rank == before.rank
                    && after.pending_offer.is_some()
            }
            (
                wire::ModificationOperation::ReshapeChoose { choice: Some(0) },
                Some(before),
                Some(after),
            ) => {
                after.level == before.level
                    && after.rank == before.rank
                    && after.entry_index
                        == before
                            .pending_offer
                            .map(|offer| offer[0])
                            .unwrap_or(u8::MAX)
                    && after.pending_offer.is_none()
            }
            (wire::ModificationOperation::Clear, Some(_), None) => true,
            _ => false,
        };
        observed.require(
            "4-slot-transition",
            transition_matches,
            (operation, previous_slot, reply),
        );
        observed.require(
            "4-cost-delta",
            false,
            format!(
                "operation={operation:?} reply={reply:?}; no player inventory/currency state is exposed by the native proficiency replies"
            ),
        );
        previous_slot = reply.slot;
    }
    let stable = browse(&mut client, browse_offset).await?;
    let refused = modify(
        &mut client,
        definition,
        wire::ModificationOperation::RankUp,
        u64::MAX,
    )
    .await?;
    observed.require(
        "stale-refusal",
        refused.result == wire::ModificationResult::StaleRevision,
        refused,
    );
    observed.require(
        "refusal-no-delta",
        browse(&mut client, browse_offset).await? == stable,
        &stable,
    );

    full_row_p05::logout_after_combat(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xa3).await?;
    observed.require(
        "fresh-session",
        fresh.join_snapshot().game_session_id != initial_session,
        fresh.join_snapshot().game_session_id,
    );
    observed.require(
        "durable-state",
        browse(&mut fresh, browse_offset).await? == stable,
        &stable,
    );
    observed.require(
        "6-shaped-slot-catalyst-durability",
        false,
        "fresh browse matches XP/selections, but exposes neither shaped slots nor catalyst inventory consumption",
    );
    full_row_p05::logout_after_combat(&mut fresh).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let mut observed = Observations::default();
    let result = verdict::indexed_timeout(
        Duration::from_secs(240),
        verdict::observe(journey(clients, &mut observed)),
    )
    .await;
    match result {
        Ok(Err(error)) => observed.failures.push(error.to_string()),
        Err(_) => observed.failures.push("step-deadline:timeout".into()),
        Ok(Ok(())) => {}
    }
    if observed.failures.is_empty() {
        evidence("P08_GAP status=PASS");
        evidence(&format!(
            "e2e_all scenario=P08.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        ));
    } else {
        let detail = observed
            .failures
            .join(";")
            .replace(char::is_whitespace, "_");
        evidence(&format!("P08_GAP status=FAIL observed={detail}"));
        evidence(&format!(
            "e2e_all scenario=P08.full status=FAIL seconds={:.3} evidence={detail}",
            started.elapsed().as_secs_f64()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_verdict_is_derived_from_failed_steps() {
        let mut observations = Observations::default();
        observations.require("accepted", true, "accepted");
        assert!(observations.failures.is_empty());
        observations.require("refused", false, wire::ModificationResult::NoOrb);
        assert_eq!(observations.failures, ["step-refused:NoOrb"]);
    }
}

#[cfg(test)]
#[test]
fn row_08_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p08.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P08_GAP status="))
    );
}
