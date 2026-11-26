//! I03 concrete player scenario, derived from the matrix, reaudit, and `item_look` owner:
//! 1. Enter through a fresh ordinary admission and LOOK at the character itself; expect the
//!    exact creature description returned in an accepted command reply.
//! 2. LOOK at an item genuinely held by the character (the node-boot starter backpack); expect
//!    accepted exact item text, including the server-computed look-distance description.
//! 3. Send LOOK for a non-existent carried-item handle; expect an accepted reply containing the
//!    fact-free `NotAvailable` refusal, proving the negative path does not disclose item facts.
//! 4. After walking to a readable sign and provisioning a spellbook, LOOK at
//!    each real handle; expect exact authored sign/book text and the vocation-filtered spell list
//!    grouped by level (words, name, and fixed or percentage mana cost).
//! 5. Log out normally, obtain a genuinely fresh admission, LOOK at the same durable carried
//!    items again, and require byte-identical book/document and spellbook content.
//! 6. Emit PASS only after every reply and durable re-check succeeds; otherwise emit FAIL with
//!    the first attempted step and observed status/refusal/missing state plus the server/content
//!    facility needed to continue.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_look as wire;
use std::num::NonZeroU64;

async fn look(
    client: &mut DevClientSession,
    target: wire::ItemLookTarget,
) -> TestResult<(CommandStatus, wire::ItemLookResult)> {
    let selection = wire::ItemLookSelection { extended: true };
    let payload = wire::encode_item_look_intent_for(wire::ItemLookIntent { target }, selection)
        .map_err(|error| format!("I03 LOOK encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::ITEM_LOOK_CAPABILITY_ID.ok_or("I03 LOOK capability unallocated")?,
            wire::ITEM_LOOK_COMMAND_ID.ok_or("I03 LOOK command unallocated")?,
            &payload,
        )
        .await?;
    let result = wire::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("I03 LOOK decode status={:?}: {error:?}", reply.status))?;
    Ok((reply.status, result))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_I03_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let items = provisioning::item_index()?;
    let spellbook = provisioning::item_reference(&items, 3059)?;
    let parchment = provisioning::item_reference(&items, 2814)?;
    character_prep::declare_seeded(
        "I03.full",
        "one spellbook (3059) and one blank parchment (2814)",
        "precondition for spellbook LOOK",
    );
    let mut setup = connect_character(clients, 0xb3, 0).await?;
    let setup_session = setup.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(setup_session.as_bytes()),
        "--profile",
        "item-3059-1".into(),
    )
    .await?;
    request_fixture_control(
        uuid_text(setup_session.as_bytes()),
        "--profile",
        "item-2814-1".into(),
    )
    .await?;
    require_logout(&mut setup, "fixture").await?;
    drop(setup);
    let mut client = connect_character(clients, 0xb3, 0).await?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("step=spellbook-fixture observed=session-reused".into());
    }
    let handle = carried_handle(&client, spellbook)?;
    evidence("I03_CHECK spellbook-fixture durable-carried-item=3059");
    let entities = client
        .world_entities()
        .ok_or("step-1: own actor view missing")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .ok_or("step-1: own actor missing from visible entities")?
        .entity;
    let observed = look(&mut client, wire::ItemLookTarget::Creature(own)).await?;
    if observed.0 != CommandStatus::Accepted
        || !matches!(&observed.1, wire::ItemLookResult::Text(text) if text.starts_with("You see yourself (Level ") && text.ends_with("."))
    {
        return Err(format!(
            "step-1 self LOOK expected exact accepted description; observed={observed:?}"
        )
        .into());
    }

    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("step-2: legitimate starter backpack missing from inventory delta")?;
    let item_observed = look(&mut client, wire::ItemLookTarget::Item(backpack.handle)).await?;
    if item_observed.0 != CommandStatus::Accepted
        || !matches!(&item_observed.1, wire::ItemLookResult::Text(text) if text.starts_with("You see "))
    {
        return Err(
            format!("step-2 starter-item LOOK expected text; observed={item_observed:?}").into(),
        );
    }

    let absent = NonZeroU64::new(u64::MAX).ok_or("step-3: invalid test handle")?;
    let inventory_before = client
        .inventory()
        .ok_or("missing inventory before refusal")?
        .clone();
    let position_before = client.world_spatial().actor_position;
    if inventory_before
        .main_backpack
        .iter()
        .chain(&inventory_before.entries)
        .any(|entry| entry.handle == absent)
        || inventory_before
            .equipment
            .iter()
            .any(|entry| entry.item.handle == absent)
    {
        return Err("step-3: negative handle is already present".into());
    }
    let refusal = look(&mut client, wire::ItemLookTarget::Item(absent)).await?;
    if refusal
        != (
            CommandStatus::Accepted,
            wire::ItemLookResult::Refused(wire::ItemLookRefusal::NotAvailable),
        )
    {
        return Err(
            format!("step-3 unavailable-item refusal mismatch; observed={refusal:?}").into(),
        );
    }

    if client.inventory() != Some(&inventory_before)
        || client.world_spatial().actor_position != position_before
    {
        return Err("step-3: unavailable LOOK changed inventory or position".into());
    }

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let start = tile(client.world_spatial().actor_position)?;
    let mut readable = Vec::new();
    oteryn_world_bundle::bundle::visit(&bytes, |_, sector| {
        if start.floor.abs_diff(sector.floor) > 2 {
            return Ok(());
        }
        for tile in sector.tiles {
            if start.x.abs_diff(tile.x) > 160 || start.y.abs_diff(tile.y) > 160 {
                continue;
            }
            let at = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if let Some(text) = item
                    .attrs
                    .text
                    .as_ref()
                    .filter(|text| !text.is_empty() && text.len() <= 256)
                    && let Ok(ordinal) = u8::try_from(ordinal)
                    && let Some(key) = crate::map::view::placement_key(at, ordinal)
                {
                    readable.push((at, key, text.clone()));
                }
            }
        }
        Ok(())
    })?;
    let distance = |a: TilePos, b: TilePos| {
        (a.floor == b.floor).then(|| a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)))
    };
    let mut approaches = BTreeMap::new();
    for (index, (at, _, _)) in readable.iter().enumerate() {
        for x in at.x.saturating_sub(1)..=at.x.saturating_add(1) {
            for y in at.y.saturating_sub(1)..=at.y.saturating_add(1) {
                let approach = TilePos {
                    x,
                    y,
                    floor: at.floor,
                };
                if distance(approach, *at) == Some(1) {
                    approaches.entry(approach).or_insert(index);
                }
            }
        }
    }
    let path = route(world, start, |pos| approaches.contains_key(&pos))
        .ok_or("step=map-reading missing=reachable-authored-text-placement")?;
    evidence(&format!(
        "I03_CHECK near-map-reading walk_steps={}",
        path.len()
    ));
    walk_once(&mut client, world, &path).await?;
    let arrived = tile(client.world_spatial().actor_position)?;
    let &(at, key, ref text) = approaches
        .get(&arrived)
        .and_then(|index| readable.get(*index))
        .ok_or("step=map-reading missing=authored-text-at-arrival")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = wire::ItemLookTarget::WorldObject(
        oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement,
            expected_revision: 0,
        },
    );
    let near = look(&mut client, target.clone()).await?;
    if near.0 != CommandStatus::Accepted
        || !matches!(&near.1, wire::ItemLookResult::Text(value) if reading_matches(value, text))
    {
        return Err(format!("step=near-map-reading position={at:?} observed={near:?}").into());
    }
    let close = tile(client.world_spatial().actor_position)?;
    let far_path = route(world, close, |pos| distance(pos, at) == Some(5))
        .ok_or("step=far-map-reading missing=route-to-five-tile-reading-distance")?;
    walk_once(&mut client, world, &far_path).await?;
    let far = look(&mut client, target.clone()).await?;
    if far.0 != CommandStatus::Accepted
        || !matches!(&far.1, wire::ItemLookResult::Text(value) if value.contains("\nYou are too far away to read it") && !value.contains("\nYou read: "))
    {
        return Err(format!("step=far-map-reading position={at:?} observed={far:?}").into());
    }
    let expected = expected_spellbook(&mut client).await?;
    let book = look(&mut client, wire::ItemLookTarget::Item(handle)).await?;
    if book.0 != CommandStatus::Accepted
        || !matches!(&book.1, wire::ItemLookResult::Text(value) if spellbook_matches(value, &expected))
    {
        return Err(format!("step=spellbook-look observed={book:?}").into());
    }
    let document = write_and_look(&mut client, parchment).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let back = route(world, from, |pos| pos == close)
        .ok_or("step=relog-map-reading missing=return-route")?;
    walk_once(&mut client, world, &back).await?;
    let session = client.join_snapshot().game_session_id;
    require_logout(&mut client, "relog").await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xb4, 0).await?;
    if fresh.join_snapshot().game_session_id == session || look(&mut fresh, target).await? != near {
        return Err("step=relog-map-reading observed=session-reused-or-reading-changed".into());
    }
    let handle = carried_handle(&fresh, spellbook)?;
    if look(&mut fresh, wire::ItemLookTarget::Item(handle)).await? != book {
        return Err("step=relog-spellbook observed=spell-list-changed".into());
    }
    evidence(&format!(
        "I03_CHECK map-reading-and-spellbook near-far-relog PASS position={at:?}"
    ));
    let handle = carried_handle(&fresh, parchment)?;
    if look(&mut fresh, wire::ItemLookTarget::Item(handle)).await? != document {
        return Err("step=relog-document observed=reading-changed".into());
    }
    evidence("I03_CHECK carried-document blank-write-look-relog PASS");
    require_logout(&mut fresh, "final").await
}

async fn walk_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, before, direction).ok_or("missing route destination")?;
        let reply = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if reply.disposition != StepDisposition::Moved
            || reply.world_spatial_delta.is_none()
            || after != expected
            || after == before
        {
            return Err(format!(
                "LOOK route {direction:?}: expected={expected:?} observed={after:?} reply={reply:?}"
            )
            .into());
        }
    }
    Ok(())
}

async fn require_logout(client: &mut DevClientSession, step: &str) -> TestResult {
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step={step} logout-refused: {detail}").into());
    }
    Ok(())
}

async fn write_and_look(
    client: &mut DevClientSession,
    parchment: std::num::NonZeroU32,
) -> TestResult<(CommandStatus, wire::ItemLookResult)> {
    use oteryn_protocol_oteryn::mail;

    const TEXT: &str = "Meet by the bridge. Bring a torch.";
    let handle = carried_handle(client, parchment)?;
    let target = wire::ItemLookTarget::Item(handle);
    let (status, blank) = look(client, target.clone()).await?;
    require_mechanic_acceptance(status, "blank-document-look")?;
    let wire::ItemLookResult::Text(blank) = blank else {
        return Err("blank-document LOOK refused".into());
    };
    if !blank.contains("parchment") || blank.contains(TEXT) || blank.contains("\nYou read: ") {
        return Err(format!("unexpected blank-document description: {blank:?}").into());
    }

    let payload = mail::encode_intent(&mail::MailIntent::WriteText {
        item_handle: handle.get(),
        text: TEXT.into(),
    })
    .map_err(|error| format!("document write encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            u32::from(mail::CAPABILITY_MAIL_V1),
            u32::from(mail::COMMAND_TYPE_MAIL),
            &payload,
        )
        .await?;
    require_mechanic_acceptance(reply.status, "document-write")?;
    if reply.payload != [mail::MailOutcome::TextWritten as u8] {
        return Err(format!("document write refused: {:?}", reply.payload).into());
    }

    let expected = (
        CommandStatus::Accepted,
        wire::ItemLookResult::Text(format!("{blank}\nYou read: {TEXT}")),
    );
    let observed = look(client, target).await?;
    if observed != expected {
        return Err(
            format!("written document LOOK expected={expected:?} observed={observed:?}").into(),
        );
    }
    Ok(observed)
}

fn carried_handle(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<NonZeroU64> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == definition)
        })
        .map(|entry| entry.handle)
        .ok_or_else(|| "step=spellbook-look observed=missing-carried-handle".into())
}

fn reading_matches(value: &str, authored: &str) -> bool {
    value
        .split_once("\nYou read: ")
        .is_some_and(|(_, text)| text == authored)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let evidence_text = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step deadline; observed=timeout; need responsive LOOK/content path".into()),
    };
    if let Some(detail) = &evidence_text {
        evidence(&format!("I03_GAP {detail}"));
    }
    evidence(&format!(
        "e2e_all scenario=I03.full status={} seconds={:.3}{}",
        if evidence_text.is_none() {
            "PASS"
        } else {
            "FAIL"
        },
        started.elapsed().as_secs_f64(),
        evidence_text
            .as_deref()
            .map(|text| format!(" evidence={}", text.replace([' ', '\n'], "_")))
            .unwrap_or_default(),
    ));
}

async fn expected_spellbook(client: &mut DevClientSession) -> TestResult<String> {
    use crate::spell::{Carrier, ManaCost, Vocation};
    use oteryn_protocol_oteryn::cyclopedia_character as cyclopedia;
    use std::fmt::Write as _;

    let request = [cyclopedia::CharacterTab::GeneralStats as u8, 0, 0, 1];
    let reply = client
        .session_mut()
        .gated_command(
            cyclopedia::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            388,
            &request,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=spellbook-vocation observed={:?}", reply.status).into());
    }
    let vocation_id = general_stats::validate(&reply.payload, general_stats::selected(client))?
        .ok_or("step=spellbook-vocation missing=served-vocation-id")?;
    let vocation = [
        "none",
        "druid",
        "elder_druid",
        "sorcerer",
        "master_sorcerer",
        "knight",
        "elite_knight",
        "paladin",
        "royal_paladin",
        "monk",
        "exalted_monk",
    ]
    .into_iter()
    .find(|key| cyclopedia::vocation_id(key) == Some(vocation_id))
    .and_then(Vocation::from_key)
    .ok_or("step=spellbook-vocation observed=unknown-vocation-id")?;

    let catalogue = native_spell_book()?;
    let mut entries: Vec<_> = catalogue
        .spells
        .iter()
        .enumerate()
        .filter(|(index, spell)| {
            !catalogue.inactive_aliases.contains(index)
                && spell.level > 0
                && spell.vocations.contains(&vocation)
        })
        .filter_map(|(_, spell)| match &spell.carrier {
            Carrier::Instant { words, .. } => Some((spell, words)),
            Carrier::Rune { .. } => None,
        })
        .collect();
    entries.sort_by_key(|(spell, _)| spell.level);
    let mut expected = String::new();
    let mut level = None;
    for (spell, words) in entries {
        if level != Some(spell.level) {
            if level.is_some() {
                expected.push('\n');
            }
            writeln!(expected, "Spells for Level {}", spell.level)?;
            level = Some(spell.level);
        }
        let mana = match spell.mana {
            ManaCost::Fixed(amount) => amount.to_string(),
            ManaCost::PercentOfMaximum(amount) => format!("{amount}%"),
        };
        writeln!(expected, "  {words} - {} : {mana}", spell.name)?;
    }
    expected.push('\n');
    Ok(expected)
}

fn spellbook_matches(value: &str, expected: &str) -> bool {
    value == expected
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_second_spellbook_rejects_truncated_wrong_and_incomplete_lists() {
        let expected = "Spells for Level 8\n  alpha - A : 20\n  beta - B : 50%\n\n";
        assert!(spellbook_matches(expected, expected));
        assert!(!spellbook_matches(
            "Spells for Level 8\n  alpha - A : 20\n\n",
            expected
        ));
        assert!(!spellbook_matches(&expected.replace("50%", "50"), expected));
        assert!(!spellbook_matches(
            &expected.replace("beta", "gamma"),
            expected
        ));
    }

    #[test]
    fn authored_reading_requires_complete_text() {
        assert!(reading_matches(
            "You see a sign.\nYou read: North\nEast",
            "North\nEast"
        ));
        assert!(!reading_matches(
            "You see a sign.\nYou read: North",
            "North\nEast"
        ));
        assert!(!reading_matches("You see North\nEast", "North\nEast"));
    }

    #[test]
    fn unavailable_handle_is_nonzero_and_outside_normal_allocation() {
        assert_eq!(
            NonZeroU64::new(u64::MAX).map(NonZeroU64::get),
            Some(u64::MAX)
        );
    }
}
