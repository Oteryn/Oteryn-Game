//! P21 client replay capture: the real IMBUEMENT query reply for a carried item that has imbuement
//! slots and offers, and the server's answer to a query for an item that is not carried. The
//! consumer is `apply_result` and `panel_text` in `crates/client-ui/src/features/0068-imbue`.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::imbuement::{
    CAPABILITY_IMBUE_V1, CAPABILITY_IMBUEMENT_SCROLL_CATALOGUE_V1,
    CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1, COMMAND_TYPE_IMBUEMENT, ImbuementAction,
    ImbuementOffer, ImbuementRequest, ImbuementResult, ImbuementWindow,
    decode_imbuement_reply_with_success, encode_imbuement_request,
};
use oteryn_protocol_oteryn::item_view::{
    CharacterInventory, encode_character_inventory_with_appearances,
};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
use std::num::NonZeroU64;

const SWORD: u32 = 3264;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let root = required("E2E_ALL_CAPTURE_DIR")?;
    std::fs::write(Path::new(&root).join(name), bytes)?;
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P21 logout refused: {:?}", outcome.result).into());
    }
    Ok(())
}

// Sword eligibility: rulesets/items/imbuements/equipment.json (wiki_swords, item 3264).
// Global fees: https://tibia.fandom.com/wiki/Imbuing
fn expected_offers() -> Vec<ImbuementOffer> {
    let families = [1, 11, 12, 20, 21, 22, 23, 24, 25];
    families
        .iter()
        .flat_map(|&family| {
            [(1, 7_500, false), (2, 60_000, true), (3, 250_000, true)].map(
                |(tier, gold, requires_premium)| ImbuementOffer {
                    family,
                    tier,
                    gold,
                    requires_premium,
                },
            )
        })
        .collect()
}

fn require_imbuement_capabilities(selected: &[u32]) -> TestResult {
    for required in [
        CAPABILITY_IMBUE_V1,
        CAPABILITY_IMBUEMENT_SCROLL_CATALOGUE_V1,
        CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1,
    ] {
        if !selected.contains(&required) {
            return Err(format!("P21 admission omitted capability {required}").into());
        }
    }
    Ok(())
}

fn verify_window(window: &ImbuementWindow, success: &[u8]) -> TestResult {
    let offers = expected_offers();
    if window.slot_count != 2
        || window.clear_gold != 15_000
        || !window.applied.is_empty()
        || window.offers != offers
    {
        return Err(format!("P21 Sword catalogue mismatch: {window:?}").into());
    }
    if success.len() != offers.len() || success.iter().any(|&percent| percent != 100) {
        return Err(format!("P21 Sword success percentages mismatch: {success:?}").into());
    }
    Ok(())
}

fn verify_inventory(
    before: &CharacterInventory,
    observed: Option<&CharacterInventory>,
    stage: &str,
) -> TestResult {
    if observed != Some(before) {
        return Err(format!("P21 inventory changed at {stage}").into());
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn query(
    client: &mut DevClientSession,
    item: NonZeroU64,
) -> TestResult<(CommandStatus, u64, Vec<u8>, Vec<u8>)> {
    let payload = encode_imbuement_request(&ImbuementRequest {
        item,
        action: ImbuementAction::Query,
    })
    .map_err(|error| format!("request encoder: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_IMBUE_V1, COMMAND_TYPE_IMBUEMENT, &payload)
        .await?;
    Ok((reply.status, reply.server_sequence, reply.payload, payload))
}

async fn require_shrine_picker(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    let world = full_e2ef7::qualification_world(clients)?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let bundle = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let shrine_palette: BTreeSet<usize> = bundle
        .manifest
        .palette
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            [
                24964, 25060, 25061, 25103, 25104, 25174, 25175, 25182, 25183, 25202,
            ]
            .iter()
            .any(|id| entry.key == format!("oteryn:item.tibia.i{id}"))
        })
        .map(|(index, _)| index)
        .collect();
    let mut shrines = BTreeSet::new();
    oteryn_world_bundle::bundle::visit(&bytes, |_, sector| {
        for tile in &sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if shrine_palette.contains(&(item.palette as usize))
                    && let Ok(ordinal) = u8::try_from(ordinal)
                    && let Some(key) = oteryn_world_bundle::bundle::placement_key(
                        sector.floor,
                        tile.x,
                        tile.y,
                        ordinal,
                    )
                    && !world.facts().base_hidden(key)
                {
                    shrines.insert((sector.floor, tile.x, tile.y, key));
                }
            }
        }
        Ok(())
    })?;
    let adjacent = |at: TilePos| {
        shrines.iter().find(|&&(floor, x, y, _)| {
            floor == at.floor && x.abs_diff(at.x).max(y.abs_diff(at.y)) <= 1
        })
    };
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        adjacent(at).is_some()
    })
    .ok_or("P21 no reachable authored imbuing shrine")?;
    walk(client, world, &path).await?;
    let &(floor, x, y, key) = adjacent(tile(client.world_spatial().actor_position)?)
        .ok_or("P21 did not reach the authored imbuing shrine")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let result = client.session_mut().use_object(&placement, 0).await?;
    evidence(&format!(
        "P21_SHRINE_USE x={x} y={y} floor={floor} placement={key} disposition={:?}",
        result.disposition
    ));
    // USE_INTENT has no imbuing-picker result or event yet. A generic Use acknowledgement
    // gives no evidence that the shrine opened the picker, so block before sending Query.
    Err(format!(
        "BLOCKER-SERVER P21 shrine Use has no item-picker event: {:?}",
        result.disposition
    )
    .into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    character_prep::declare_seeded(
        "P21.client",
        "sword-3264",
        "the row exercises imbuement queries and client rendering",
    );
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_P21_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let mut client = connect_character(&scope, 0x46, 0).await?;
    let definitions = provisioning::item_index()?;
    let definition = provisioning::item_reference(&definitions, SWORD)?;
    let before = client
        .inventory()
        .ok_or("fixture inventory frame missing")?;
    if before
        .entries
        .iter()
        .chain(before.main_backpack.iter())
        .chain(before.equipment.iter().map(|entry| &entry.item))
        .any(|entry| entry.item_definition_ref == definition)
    {
        return Err("P21 sword already present before fixture grant".into());
    }
    let setup_session = client.join_snapshot().game_session_id;
    character_prep::grant_item(&client, SWORD, 1).await?;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(&scope, 0x46, 0).await?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("P21 reused the fixture grant session".into());
    }
    require_imbuement_capabilities(client.selected_capabilities())?;
    require_shrine_picker(&mut client, &scope).await?;
    let decode = |payload: &[u8]| {
        decode_imbuement_reply_with_success(payload)
            .map_err(|error| format!("P21 imbuement reply decode: {error:?}"))
    };
    let inventory = client
        .inventory()
        .cloned()
        .ok_or("inventory frame missing")?;
    let mut swords = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition);
    let sword = swords.next().ok_or("fixture sword missing after relog")?;
    if sword.count.get() != 1 || swords.next().is_some() {
        return Err("P21 expected exactly one fixture sword".into());
    }
    let handle = sword.handle;
    let (status, window_sequence, payload, request) = query(&mut client, handle).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("fixture sword query refused: {status:?}").into());
    }
    let (reply, success) = decode(&payload)?;
    let found = reply.window.ok_or("fixture sword query has no window")?;
    if reply.result != ImbuementResult::Ok {
        return Err(format!("fixture sword query result: {:?}", reply.result).into());
    }
    verify_window(&found, &success)?;
    client.service_liveness(Duration::from_millis(50)).await?;
    verify_inventory(&inventory, client.inventory(), "after-query")?;
    capture("p21-window.bin", &payload)?;
    let missing = NonZeroU64::new(u64::MAX).ok_or("nonzero handle")?;
    let (status, refused_sequence, refused, _) = query(&mut client, missing).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("unknown item query failed at command boundary: {status:?}").into());
    }
    let (refusal, _) = decode(&refused)?;
    if refusal.result != ImbuementResult::NotAllowed || refusal.window.is_some() {
        return Err(format!("unexpected unknown item query result: {refusal:?}").into());
    }
    client.service_liveness(Duration::from_millis(50)).await?;
    verify_inventory(&inventory, client.inventory(), "after-refusal")?;
    capture("p21-refused.bin", &refused)?;
    if window_sequence == 0 || refused_sequence <= window_sequence {
        return Err("P21 result sequences are not increasing".into());
    }
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let query_hex = hex(&request);
    let inventory_hex = hex(
        &encode_character_inventory_with_appearances(&inventory, true, true)
            .map_err(|error| format!("P21 inventory capture: {error:?}"))?,
    );
    let window_sha256 = hex(&Sha256::digest(&payload));
    let refused_sha256 = hex(&Sha256::digest(&refused));
    let refused_result = refusal.result;
    let mut expected = format!(
        "session={session}\nwindow_sequence={window_sequence}\nrefused_sequence={refused_sequence}\nwindow_sha256={window_sha256}\nrefused_sha256={refused_sha256}\nquery_hex={query_hex}\nitem_handle={handle}\nitem_definition={definition}\ninventory_hex={inventory_hex}\nsuccess_selected=true\nslot_count={}\nclear_gold={}\napplied={}\noffers={}\n",
        found.slot_count,
        found.clear_gold,
        found.applied.len(),
        found.offers.len()
    );
    for (index, offer) in found.offers.iter().enumerate() {
        expected.push_str(&format!(
            "offer_{index}={},{},{},{}\n",
            offer.family, offer.tier, offer.gold, offer.requires_premium
        ));
    }
    for (index, percent) in success.iter().enumerate() {
        expected.push_str(&format!("offer_{index}_success={percent}\n"));
    }
    expected.push_str(&format!(
        "refused_status={status:?}\nrefused_result={refused_result:?}\n"
    ));
    capture("p21-expected.txt", expected.as_bytes())?;
    let query_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&scope, 0x46, 0).await?;
    if fresh.join_snapshot().game_session_id == query_session {
        return Err("P21 reused the query session".into());
    }
    let reloaded = fresh
        .inventory()
        .cloned()
        .ok_or("post-query relog inventory missing")?;
    require_imbuement_capabilities(fresh.selected_capabilities())?;
    let sword = reloaded
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == definition)
        .ok_or("P21 Sword missing from post-query admission")?;
    let (status, _, payload, _) = query(&mut fresh, sword.handle).await?;
    let (reply, success) = decode(&payload)?;
    if status != CommandStatus::Accepted || reply.result != ImbuementResult::Ok {
        return Err(format!(
            "P21 post-relog Sword query refused: {status:?} {:?}",
            reply.result
        )
        .into());
    }
    verify_window(
        &reply.window.ok_or("P21 post-relog Sword window missing")?,
        &success,
    )?;
    verify_inventory(
        &full_row_p18::durable_inventory(inventory),
        Some(&full_row_p18::durable_inventory(reloaded)),
        "after-relog",
    )?;
    evidence(
        "P21_CHECK inventory PASS checkpoints=before-query,after-query,after-refusal,after-relog,after-relog-sword-query",
    );
    logout(&mut fresh).await?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P21.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P21.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P21.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;

    #[test]
    fn p21_catalogue_rejects_wrong_clear_fee_offers_and_success() -> TestResult {
        let correct = ImbuementWindow {
            slot_count: 2,
            clear_gold: 15_000,
            applied: Vec::new(),
            offers: expected_offers(),
        };
        let success = vec![100; 27];
        verify_window(&correct, &success)?;
        let mut wrong = correct.clone();
        wrong.clear_gold += 1;
        assert!(verify_window(&wrong, &success).is_err());
        let changes: [fn(&mut Vec<ImbuementOffer>); 6] = [
            |offers: &mut Vec<ImbuementOffer>| {
                offers.truncate(26);
            },
            |offers: &mut Vec<ImbuementOffer>| {
                offers.push(ImbuementOffer {
                    family: 26,
                    tier: 1,
                    gold: 7_500,
                    requires_premium: false,
                });
            },
            |offers: &mut Vec<ImbuementOffer>| {
                offers[0].family = 4;
            },
            |offers: &mut Vec<ImbuementOffer>| {
                offers[0].tier = 3;
            },
            |offers: &mut Vec<ImbuementOffer>| {
                offers[0].gold += 1;
            },
            |offers: &mut Vec<ImbuementOffer>| {
                offers[0].requires_premium = true;
            },
        ];
        for change in changes {
            let mut wrong = correct.clone();
            change(&mut wrong.offers);
            assert!(verify_window(&wrong, &success).is_err());
        }
        let mut wrong_success = success;
        wrong_success[0] = 99;
        assert!(verify_window(&correct, &wrong_success).is_err());
        assert!(verify_window(&correct, &[]).is_err());
        let mut persisted = correct;
        persisted
            .applied
            .push(oteryn_protocol_oteryn::imbuement::AppliedImbuement {
                slot: 0,
                family: 1,
                tier: 1,
                remaining_ms: 72_000_000,
            });
        assert!(verify_window(&persisted, &[100; 27]).is_err());
        Ok(())
    }

    #[test]
    fn p21_requires_both_catalogue_extensions() -> TestResult {
        let required = [
            CAPABILITY_IMBUE_V1,
            CAPABILITY_IMBUEMENT_SCROLL_CATALOGUE_V1,
            CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1,
        ];
        require_imbuement_capabilities(&required)?;
        for omitted in required {
            let selected: Vec<_> = required.into_iter().filter(|id| *id != omitted).collect();
            assert!(require_imbuement_capabilities(&selected).is_err());
        }
        Ok(())
    }

    #[test]
    fn p21_inventory_rejects_mutations_visible_only_on_relog() -> TestResult {
        use oteryn_protocol_oteryn::item_view::ItemEntry;
        let before = CharacterInventory {
            entries: vec![ItemEntry {
                appearance_id: 0,
                handle: NonZeroU64::MIN,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..CharacterInventory::default()
        };
        verify_inventory(&before, Some(&before), "after-query")?;
        verify_inventory(&before, Some(&before), "after-refusal")?;
        let mut changed = before.clone();
        changed.entries[0].count = std::num::NonZeroU32::MAX;
        for checkpoint in ["after-query", "after-refusal"] {
            assert!(verify_inventory(&before, Some(&changed), checkpoint).is_err());
        }
        let mut fresh = before.clone();
        fresh.entries[0].handle = NonZeroU64::MAX;
        verify_inventory(
            &before,
            Some(&full_row_p18::durable_inventory(fresh.clone())),
            "after-relog",
        )?;
        fresh.entries[0].count = std::num::NonZeroU32::MAX;
        assert!(
            verify_inventory(
                &before,
                Some(&full_row_p18::durable_inventory(fresh)),
                "after-relog"
            )
            .is_err()
        );
        Ok(())
    }
}
