//! W19d ordinary USE of carried outfit acquisition items.
use super::*;
use crate::content::native_gameplay::{NativeCosmeticsView, test_support::cosmetics_fixture};
use crate::domain::appearance::{AccountUnlocks, EarnedCosmetics, Sex, UnlockFact};
use oteryn_dev_client::DevClientSession;
use oteryn_gameplay_items::item_cosmetics;
use oteryn_protocol_oteryn::{appearance, world_object as use_wire};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

const SPARKION_TOKEN: u32 = 23538;
const SPARKION_MOUNT: &str = "oteryn:content.mount.sparkion_mount";
const ADDON_DOLL: u32 = 6579;
const AFFLICTED_OUTFIT_PIECE: u32 = 12551;
const PROBES: [(&str, u32); 2] = [
    ("addon doll", ADDON_DOLL),
    ("afflicted outfit item", AFFLICTED_OUTFIT_PIECE),
];
const AFFLICTED_PIECES: [u32; 6] = [12551, 12552, 12553, 12554, 12555, 12556];
const DOLL_COMMAND_WINDOW: u64 = 1024;
const NEEDED: &str = "needed:outfit-or-mount-item-6579-or-12551-in-inventory;no-npc-seller(content/services/trade);drops-only-from-high-level-creatures(content/loot)";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    // The duplicate can require hundreds of read-only exchanges before its single USE.
    let result = verdict::indexed_timeout(Duration::from_secs(900), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "afflicted-six-pieces-addon-doll-and-sparkion-mount-durable".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W19d.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(400)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut result = BTreeMap::new();
    for entry in &client
        .inventory()
        .ok_or("step=admit observed=no-inventory")?
        .entries
    {
        *result.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    Ok(result)
}

async fn use_by_definition(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<(CommandStatus, use_wire::UseDisposition)> {
    let payload = use_wire::encode_item_use_intent(&use_wire::UseIntent {
        target: use_wire::UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=use observed=encode-{error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (body, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("step=refused-use observed=refusal-detail-{error:?}"))?;
    let disposition = use_wire::decode_use_result(body)
        .map_err(|error| format!("step=refused-use observed=decode-{error:?}"))?;
    Ok((reply.status, disposition))
}

async fn relog(
    client: DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let old_session = client.join_snapshot().game_session_id;
    logout(client).await?;
    let fresh = connect_character(clients, 0xd4, 0).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("step=relog observed=reused-session".into());
    }
    Ok(fresh)
}

async fn logout(mut client: DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_once(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={detail}").into());
    }
    Ok(())
}

async fn verify_male_catalogue(
    clients: &SeamClients<'_>,
    previous_session: crate::foundation::GameSessionId,
    expected: &BTreeMap<String, (u32, u8)>,
    mounts: &[String],
) -> TestResult<crate::foundation::GameSessionId> {
    let mut witness = connect_character(clients, 0xd4, 0).await?;
    let session = witness.join_snapshot().game_session_id;
    if session == previous_session
        || &outfits(&mut witness)? != expected
        || appearance_snapshot(&mut witness)?.catalogue.mounts != mounts
    {
        return Err("step=male-outfit-catalogue observed=wrong-outfit-or-mounts".into());
    }
    logout(witness).await?;
    Ok(session)
}

fn appearance_snapshot(client: &mut DevClientSession) -> TestResult<appearance::Snapshot> {
    let state = client
        .session_mut()
        .gated_snapshot(appearance::STATE_DOMAIN_APPEARANCE)
        .ok_or("step=appearance observed=no-admission-catalogue")?;
    appearance::Snapshot::decode(&state.payload)
        .map_err(|error| format!("step=appearance observed={error:?}").into())
}

fn outfits(client: &mut DevClientSession) -> TestResult<BTreeMap<String, (u32, u8)>> {
    Ok(appearance_snapshot(client)?
        .catalogue
        .outfits
        .into_iter()
        .map(|entry| (entry.key, (entry.look_type, entry.permitted_addons)))
        .collect())
}

async fn expect_item_use(
    client: &mut DevClientSession,
    reference: NonZeroU32,
    expected: use_wire::UseDisposition,
) -> TestResult {
    let mut candidates = client
        .inventory()
        .ok_or("step=use observed=no-inventory")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == reference);
    let handle = candidates
        .next()
        .ok_or("step=use observed=no-handle")?
        .handle;
    if candidates.next().is_some() {
        return Err("step=use observed=ambiguous-handle".into());
    }
    let reply = client.use_item(handle).await?;
    require_mechanic_acceptance(reply.status, "W19d item USE")?;
    if reply.disposition != expected {
        return Err(format!("step=use reference={reference} observed={reply:?}").into());
    }
    evidence(&format!(
        "W19D_USE reference={reference} status=Accepted disposition={expected:?} sequence={}",
        reply.result_server_sequence
    ));
    Ok(())
}

fn outfit_grant_projection(
    content: &NativeCosmeticsView,
    before: &BTreeMap<String, (u32, u8)>,
    grants: &[String],
    sex: Sex,
) -> BTreeMap<String, (u32, u8)> {
    let facts: Vec<_> = grants
        .iter()
        .map(|key| (key.clone(), item_cosmetics::PROVENANCE.to_owned()))
        .collect();
    let earned = EarnedCosmetics {
        content,
        facts: &facts,
    }
    .with_sex(sex);
    let mut expected = before.clone();
    for key in content.outfit_keys() {
        let key = key.to_owned();
        let prior = before.get(&key);
        if prior.is_none() && earned.outfit(&key) != UnlockFact::Compatible {
            continue;
        }
        let earned_addons = u8::from(earned.addon(&key, 1) == UnlockFact::Compatible)
            | (u8::from(earned.addon(&key, 2) == UnlockFact::Compatible) << 1);
        if let Some(look) = content.look_type(&key, sex) {
            expected.insert(
                key,
                (look, prior.map_or(0, |value| value.1) | earned_addons),
            );
        }
    }
    expected
}

async fn doll_source(
    clients: &SeamClients<'_>,
) -> TestResult<[u8; 16]> {
    use sqlx::Connection as _;

    // Read only the input instance identity. The wire inventory and catalogue remain
    // the sole source of observed consumption and grant evidence.
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let sources: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id) FROM game_item_container_entries e
         JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
         WHERE e.character_id = encode($1, 'hex')::uuid AND e.world_id = encode($2, 'hex')::uuid
           AND i.world_id = e.world_id AND i.lifecycle = 1 AND i.quantity = 1
           AND i.definition_family = 'Item' AND i.definition_production_key = 'oteryn:item.tibia.i6579'
         LIMIT 2",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .fetch_all(&mut db)
    .await?;
    let [source] = sources.as_slice() else {
        return Err("step=doll observed=missing-or-ambiguous-input-instance".into());
    };
    Ok(source.as_slice().try_into()?)
}

fn expected_doll_choice(
    client: &DevClientSession,
    source: [u8; 16],
    command: u64,
) -> TestResult<&'static item_cosmetics::Choice> {
    let rule =
        item_cosmetics::rule("oteryn:item.tibia.i6579").ok_or("step=doll observed=no-rule")?;
    let session = *client.join_snapshot().game_session_id.as_bytes();
    use sha2::{Digest, Sha256};

    // Crystal 201189f8 addon_doll.lua fixes the slot order. Native look
    // corrections are explicit here. Global Wedding has no addons.
    const SLOTS: [&str; 44] = [
        "128-136", "129-137", "130-138", "131-139", "132-140", "133-141",
        "134-142", "143-147", "144-148", "145-149", "146-150", "151-155",
        "152-156", "153-157", "154-158", "251-252", "268-269", "270-273",
        "278-279", "288-289", "324-325", "335-336", "366-367", "328-329",
        "430-431", "432-433", "463-464", "465-466", "471-472", "512-513",
        "514-516", "541-542", "574-575", "577-578", "610-618", "620-621",
        "632-633", "634-635", "636-637", "664-665", "666-667", "683-684",
        "694-695", "696-697",
    ];
    let draw = |domain: u8, upper: u64| {
        (0_u32..=u32::MAX).find_map(|counter| {
            let mut hash = Sha256::new();
            hash.update(b"oteryn:reward-bag-draw:v1");
            hash.update(session);
            hash.update(command.to_be_bytes());
            hash.update(source);
            hash.update([domain]);
            hash.update(counter.to_be_bytes());
            let bytes: [u8; 8] = hash.finalize()[..8].try_into().ok()?;
            let value = u64::from_be_bytes(bytes);
            (value < u64::MAX - u64::MAX % upper).then_some(value % upper)
        }).ok_or("step=doll observed=draw-exhausted")
    };
    let outfit = draw(224, 44)?;
    let addon = draw(225, 2)?;
    let pair = SLOTS[usize::try_from(outfit)?];
    let base = format!("oteryn:item-grant.outfit.{pair}.base");
    let mut grants = vec![base.clone()];
    let owned = if matches!(pair, "328-329" | "620-621") {
        base
    } else {
        let owned = format!("oteryn:item-grant.outfit.{pair}.addon{}", addon + 1);
        grants.push(owned.clone());
        owned
    };
    rule.choices
        .iter()
        .find(|choice| choice.owned == owned && choice.grants == grants)
        .ok_or_else(|| format!("step=doll observed=missing-or-mismapped-grant expected={owned}").into())

}

async fn plan_owned_doll_use(
    client: &mut DevClientSession,
    source: [u8; 16],
    known: &item_cosmetics::Choice,
) -> TestResult {
    use oteryn_protocol_oteryn::server_info;

    let first = client.next_command_id();
    let mut scheduled = None;
    for command in first..first + DOLL_COMMAND_WINDOW {
        if expected_doll_choice(client, source, command)?.owned == known.owned {
            scheduled = Some(command);
            break;
        }
    }
    let scheduled = scheduled.ok_or("step=doll-duplicate observed=no-owned-draw-in-window")?;
    // Schedule the command before sending any USE for this doll. Read-only catalogue
    // queries advance command IDs without consuming the inventory USE allowance.
    let query = server_info::Query::Quests;
    let payload = query.encode().ok_or("step=doll-duplicate-sequence observed=bad-query")?;
    let mut baseline = None;
    for command in first..scheduled {
        let response = client.session_mut().gated_command(
            server_info::CAPABILITY_SERVER_INFO_V1,
            server_info::COMMAND_TYPE_SERVER_INFO_QUERY,
            &payload,
        ).await?;
        require_mechanic_acceptance(response.status, "W19d read-only sequence preparation")?;
        let reply = server_info::Reply::decode(&response.payload)
            .ok_or("step=doll-duplicate-sequence observed=bad-query-response")?;
        if !reply.matches(&query)
            || baseline.as_ref().is_some_and(|prior| prior != &reply)
            || response.command_id != command
            || client.next_command_id() != command + 1
        {
            return Err(format!("step=doll-duplicate-sequence command={command} observed=unexpected-query-result").into());
        }
        baseline.get_or_insert(reply);
        if (command - first).is_multiple_of(32) {
            evidence(&format!("W19D_DRAW_PREPARATION_CHECK PASS command={command} scheduled={scheduled}"));
        }
    }
    Ok(())
}

fn verify_draw_delta(
    choice: &item_cosmetics::Choice,
    before: &BTreeMap<String, (u32, u8)>,
    after: &BTreeMap<String, (u32, u8)>,
) -> TestResult {
    let (_, suffix) = choice.owned.rsplit_once('.').ok_or("invalid draw key")?;
    let mask = match suffix {
        "base" => 0,
        "addon1" => 1,
        "addon2" => 2,
        _ => return Err("invalid draw addon".into()),
    };
    let mut changed = false;
    for (key, &(look, addons)) in after {
        let previous = before.get(key).copied();
        if previous == Some((look, addons)) {
            continue;
        }
        let prior_mask = previous.map_or(0, |(_, mask)| mask);
        if previous.is_some_and(|(prior_look, _)| prior_look != look)
            || (mask != 0 && prior_mask & mask != 0)
            || addons != prior_mask | mask
        {
            return Err(format!("step=doll observed=wrong-addon-delta key={key}").into());
        }
        changed = true;
    }
    if !changed {
        return Err("step=doll observed=no-draw-delta".into());
    }
    Ok(())
}

fn verify_doll(
    content: &NativeCosmeticsView,
    choice: &item_cosmetics::Choice,
    before: &BTreeMap<String, (u32, u8)>,
    after: &BTreeMap<String, (u32, u8)>,
) -> TestResult {
    verify_draw_delta(choice, before, after)?;
    let expected = outfit_grant_projection(content, before, &choice.grants, Sex::Female);
    if &expected == before || &expected != after {
        return Err(format!(
            "step=doll observed=unexpected-outfit-change expected={}",
            choice.owned
        )
        .into());
    }
    evidence(&format!(
        "W19D_DOLL_ASSERT grant={} consumed=1",
        choice.owned
    ));
    Ok(())
}

async fn require_feedback(
    client: &mut DevClientSession,
    before: u64,
    step: &str,
    matches_message: impl Fn(&oteryn_dev_client::ChatLine) -> bool,
) -> TestResult {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let events = client.wait_for_events(Duration::from_secs(3)).await?;
            if events.into_iter().any(|event| {
                matches!(event, oteryn_dev_client::SessionEvent::ChatLine(delta)
                    if delta.server_sequence > before
                        && matches_message(&delta.value))
            }) {
                return Ok::<(), oteryn_dev_client::DevClientError>(());
            }
        }
    })
    .await
    .map_err(|_| format!("step={step} observed=missing-exact-feedback after_sequence={before}"))??;
    Ok(())
}

async fn vortex_progress(clients: &SeamClients<'_>) -> TestResult<i64> {
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT progress FROM game_character_achievement_progress
         WHERE character_id = encode($1, 'hex')::uuid
           AND achievement_key = 'oteryn:achievement/vortex_tamer'",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_optional(&mut connection)
    .await?;
    Ok(count.unwrap_or(0))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let cosmetics = cosmetics_fixture::decode_cosmetics(&native_input()?.cosmetics.bytes)?;
    let male = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_W19D_ACCOUNT",
        Some("Outfit Catalogue Witness"),
    )
    .await?;
    let dedicated = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_W19D_ACCOUNT",
        Some("Outfit Item Tester"),
    )
    .await?;
    let male_clients = male.clients(clients);
    let clients = dedicated.clients(clients);
    let mut witness = connect_character(&male_clients, 0xd4, 0).await?;
    let mut male_expected = outfits(&mut witness)?;
    if male_expected.get("oteryn:outfit.citizen") != Some(&(128, 0)) {
        return Err("step=male-outfit-catalogue observed=wrong-starter-look".into());
    }
    let male_mounts = appearance_snapshot(&mut witness)?.catalogue.mounts;
    let male_initial_session = witness.join_snapshot().game_session_id;
    logout(witness).await?;
    let mut client = connect_character(&clients, 0xd4, 0).await?;
    let initial_inventory = quantities(&client)?;
    let initial_outfits = outfits(&mut client)?;
    let initial_mounts = appearance_snapshot(&mut client)?.catalogue.mounts;
    let index = provisioning::item_index()?;
    let reference = |id| provisioning::item_reference(&index, id);
    for (label, id) in PROBES {
        let reference = reference(id)?;
        if initial_inventory.contains_key(&reference.get()) {
            return Err(
                format!("step=refused-use observed={label}-already-carried {NEEDED}").into(),
            );
        }
        let (status, disposition) = use_by_definition(&mut client, reference).await?;
        if (status, disposition)
            != (
                CommandStatus::Accepted,
                use_wire::UseDisposition::NothingToUse,
            )
        {
            return Err(format!(
                "step=use-{label} without-item observed={status:?}/{disposition:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        if quantities(&client)? != initial_inventory
            || outfits(&mut client)? != initial_outfits
            || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
        {
            return Err(format!("step=refused-use item={label} observed=state-changed").into());
        }
    }
    let (&last_piece, partial_set) = AFFLICTED_PIECES
        .split_last()
        .ok_or("step=fund observed=empty-afflicted-set")?;
    let mut funded = initial_inventory.clone();
    for id in partial_set.iter().copied().chain([ADDON_DOLL]) {
        if funded.insert(reference(id)?.get(), 1).is_some() {
            return Err(format!("step=fund observed=preexisting-item-{id}").into());
        }
        character_prep::grant_item(&client, id, 1).await?;
    }
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != funded
        || outfits(&mut client)? != initial_outfits
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=fund observed=unexpected-carried-items-or-appearance".into());
    }
    let mut expected_outfits = initial_outfits.clone();
    for key in [
        "oteryn:content.outfit.afflicted_outfits",
        "oteryn:outfit.afflicted",
    ] {
        if expected_outfits.insert(key.to_owned(), (431, 0)).is_some() {
            return Err(format!("step=afflicted observed=outfit-already-owned key={key}").into());
        }
        if male_expected.insert(key.to_owned(), (430, 0)).is_some() {
            return Err(format!("step=afflicted observed=male-outfit-already-owned key={key}").into());
        }
    }
    let afflicted_reference = reference(AFFLICTED_OUTFIT_PIECE)?;
    let missing_reference = reference(last_piece)?;
    if funded.contains_key(&missing_reference.get()) {
        return Err("step=afflicted-incomplete observed=complete-set-already-carried".into());
    }
    character_prep::grant_premium(&client).await?;
    evidence("FIXTURE row=W19d.full input=Premium");
    expect_item_use(
        &mut client,
        afflicted_reference,
        use_wire::UseDisposition::RequirementNotMet,
    )
    .await?;
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != funded
        || outfits(&mut client)? != initial_outfits
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=afflicted-incomplete observed=state-changed".into());
    }
    evidence("W19D_AFFLICTED_INCOMPLETE_ASSERT disposition=RequirementNotMet persisted=unchanged");
    character_prep::grant_item(&client, last_piece, 1).await?;
    funded.insert(missing_reference.get(), 1);
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != funded
        || outfits(&mut client)? != initial_outfits
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=afflicted-complete-fund observed=unexpected-state".into());
    }
    expect_item_use(
        &mut client,
        afflicted_reference,
        use_wire::UseDisposition::PremiumRequired,
    )
    .await?;
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != funded
        || outfits(&mut client)? != initial_outfits
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=non-premium-afflicted observed=state-changed".into());
    }
    evidence("W19D_PREMIUM_REFUSAL_ASSERT disposition=PremiumRequired persisted=unchanged");
    // Current Premium proof is released on logout. Prepare it on the session that uses the pieces.
    character_prep::grant_premium(&client).await?;
    evidence("FIXTURE row=W19d.full input=Premium");
    let before_feedback = client.last_server_sequence();
    expect_item_use(
        &mut client,
        afflicted_reference,
        use_wire::UseDisposition::Committed,
    )
    .await?;
    require_feedback(&mut client, before_feedback, "afflicted", |line| {
        matches!(line, oteryn_dev_client::ChatLine::Local { text, .. } if text == "You have restored an outfit.")
    }).await?;
    let mut client = relog(client, &clients).await?;
    for id in AFFLICTED_PIECES {
        funded.remove(&reference(id)?.get());
    }
    let afflicted = outfits(&mut client)?;
    if afflicted != expected_outfits {
        let changed: Vec<_> = afflicted
            .iter()
            .filter(|(key, value)| expected_outfits.get(*key) != Some(*value))
            .collect();
        let missing: Vec<_> = expected_outfits
            .keys()
            .filter(|key| !afflicted.contains_key(*key))
            .collect();
        return Err(format!("step=afflicted changed={changed:?} missing={missing:?}").into());
    }
    let carried = quantities(&client)?;
    if carried != funded || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts {
        return Err(
            format!("step=afflicted observed=carried-{carried:?} expected={funded:?}").into(),
        );
    }
    evidence("W19D_AFFLICTED_ASSERT consumed=12551,12552,12553,12554,12555,12556 outfit=afflicted");
    logout(client).await?;
    let male_afflicted_session =
        verify_male_catalogue(&male_clients, male_initial_session, &male_expected, &male_mounts)
            .await?;
    evidence("W19D_MALE_AFFLICTED_ASSERT outfit=430 addons=0 mounts=unchanged");
    let mut client = connect_character(&clients, 0xd4, 0).await?;
    let doll_choice = expected_doll_choice(&client, doll_source(&clients).await?, client.next_command_id())?;
    expect_item_use(
        &mut client,
        reference(ADDON_DOLL)?,
        use_wire::UseDisposition::Committed,
    )
    .await?;
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != initial_inventory {
        return Err("step=doll observed=wrong-consumption".into());
    }
    let after_doll = outfits(&mut client)?;
    verify_doll(&cosmetics, doll_choice, &afflicted, &after_doll)?;
    if appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts {
        return Err("step=outfit-use observed=unexpected-mount-grant".into());
    }
    logout(client).await?;
    let male_after = outfit_grant_projection(&cosmetics, &male_expected, &doll_choice.grants, Sex::Male);
    verify_draw_delta(doll_choice, &male_expected, &male_after)?;
    verify_male_catalogue(&male_clients, male_afflicted_session, &male_after, &male_mounts).await?;
    evidence("W19D_MALE_DOLL_ASSERT addon=expected mounts=unchanged");
    let client = connect_character(&clients, 0xd4, 0).await?;
    let doll_reference = reference(ADDON_DOLL)?;
    let mut with_duplicate_doll = initial_inventory.clone();
    with_duplicate_doll.insert(doll_reference.get(), 1);
    character_prep::grant_item(&client, ADDON_DOLL, 1).await?;
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != with_duplicate_doll
        || outfits(&mut client)? != after_doll
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=doll-duplicate-fund observed=unexpected-state".into());
    }
    plan_owned_doll_use(&mut client, doll_source(&clients).await?, doll_choice).await?;
    if quantities(&client)? != with_duplicate_doll
        || outfits(&mut client)? != after_doll
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=doll-duplicate-sequence observed=state-changed".into());
    }
    // Global Wedding outfits have no addons. Both native draw slots grant the base.
    let owned_feedback = if doll_choice.owned.ends_with(".base") {
        "You already have this outfit."
    } else {
        "You already have this outfit with this addon."
    };
    let expected_feedback = if client.selected_capabilities().contains(
        &oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_SYSTEM_MESSAGE_V1,
    ) {
        oteryn_dev_client::ChatLine::System { text: owned_feedback.into() }
    } else {
        oteryn_dev_client::ChatLine::Private { speaker_name: "Game".into(), text: owned_feedback.into() }
    };
    let before_duplicate = client.last_server_sequence();
    expect_item_use(&mut client, doll_reference, use_wire::UseDisposition::Committed).await?;
    require_feedback(&mut client, before_duplicate, "doll-duplicate", |line| line == &expected_feedback).await?;
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != with_duplicate_doll
        || outfits(&mut client)? != after_doll
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=doll-duplicate observed=state-changed".into());
    }
    evidence(&format!("W19D_DOLL_DUPLICATE_ASSERT grant={} persisted=unchanged", doll_choice.owned));
    let mut duplicate_inputs = with_duplicate_doll;
    for id in AFFLICTED_PIECES {
        duplicate_inputs.insert(reference(id)?.get(), 1);
        character_prep::grant_item(&client, id, 1).await?;
    }
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != duplicate_inputs
        || outfits(&mut client)? != after_doll
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=afflicted-duplicate-fund observed=unexpected-state".into());
    }
    character_prep::grant_premium(&client).await?;
    evidence("FIXTURE row=W19d.full input=Premium");
    expect_item_use(
        &mut client,
        afflicted_reference,
        use_wire::UseDisposition::Committed,
    )
    .await?;
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != duplicate_inputs
        || outfits(&mut client)? != after_doll
        || appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts
    {
        return Err("step=afflicted-duplicate observed=state-changed".into());
    }
    evidence("W19D_AFFLICTED_DUPLICATE_ASSERT persisted=unchanged");
    let expected_mounts = initial_mounts;
    if expected_mounts.iter().any(|mount| mount == SPARKION_MOUNT) {
        return Err("step=mount observed=already-owned".into());
    }
    let mut mounted = expected_mounts.clone();
    mounted.push(SPARKION_MOUNT.to_owned());
    mounted.sort();
    let mount_reference = reference(SPARKION_TOKEN)?;
    let mut mount_inputs = duplicate_inputs.clone();
    if mount_inputs.insert(mount_reference.get(), 1).is_some() {
        return Err("step=mount observed=preexisting-token".into());
    }
    character_prep::grant_item(&client, SPARKION_TOKEN, 1).await?;
    let mut client = relog(client, &clients).await?;
    if quantities(&client)? != mount_inputs
        || outfits(&mut client)? != after_doll
        || appearance_snapshot(&mut client)?.catalogue.mounts != expected_mounts
    {
        return Err("step=mount-fund observed=unexpected-input-or-appearance".into());
    }
    character_prep::grant_premium(&client).await?;
    evidence("FIXTURE row=W19d.full input=Premium");
    if vortex_progress(&clients).await? != 0 { return Err("step=sparkion observed=preexisting-vortex-progress".into()); }
    let before_feedback = client.last_server_sequence();
    expect_item_use(
        &mut client,
        mount_reference,
        use_wire::UseDisposition::Committed,
    )
    .await?;
    require_feedback(&mut client, before_feedback, "sparkion", |line| {
        matches!(line, oteryn_dev_client::ChatLine::Local { text, .. } if text == "You receive the permission to ride a sparkion.")
    }).await?;
    let mut client = relog(client, &clients).await?;
    let mut observed_mounts = appearance_snapshot(&mut client)?.catalogue.mounts;
    observed_mounts.sort();
    if quantities(&client)? != duplicate_inputs
        || outfits(&mut client)? != after_doll
        || observed_mounts != mounted
    {
        return Err("step=mount observed=unexpected-persisted-outcome".into());
    }
    if vortex_progress(&clients).await? != 1 {
        return Err("step=sparkion observed=wrong-persisted-vortex-progress".into());
    }
    evidence("W19D_MOUNT_ASSERT grant=sparkion_mount consumed=23538");
    logout(client).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needed_precondition_is_one_report_token() {
        assert!(NEEDED.starts_with("needed:"));
        assert!(!compact(NEEDED).contains(char::is_whitespace));
    }

    #[test]
    fn doll_requires_one_new_donor_addon_and_preserves_existing_outfits() -> TestResult {
        let cosmetics = cosmetics_fixture::decode_cosmetics(&native_input()?.cosmetics.bytes)?;
        let citizen = "oteryn:outfit.citizen".to_owned();
        let citizen_alias = "oteryn:content.outfit.citizen_outfits".to_owned();
        let afflicted = "oteryn:outfit.afflicted".to_owned();
        let afflicted_alias = "oteryn:content.outfit.afflicted_outfits".to_owned();
        let before = BTreeMap::from([
            (citizen.clone(), (136, 0)),
            (citizen_alias.clone(), (136, 0)),
        ]);
        let rule = item_cosmetics::rule("oteryn:item.tibia.i6579").ok_or("doll rule")?;
        let choice = |key| {
            rule.choices
                .iter()
                .find(|choice| choice.owned == key)
                .ok_or("authored choice")
        };
        let citizen_choice = choice("oteryn:item-grant.outfit.128-136.addon1")?;
        let afflicted_choice = choice("oteryn:item-grant.outfit.430-431.addon2")?;
        let existing_base = BTreeMap::from([
            (citizen.clone(), (136, 1)),
            (citizen_alias.clone(), (136, 1)),
        ]);
        verify_doll(&cosmetics, citizen_choice, &before, &existing_base)?;
        let mut missing_alias = existing_base.clone();
        missing_alias.insert(citizen_alias, (136, 0));
        assert!(verify_doll(&cosmetics, citizen_choice, &before, &missing_alias).is_err());
        let mut new_base = before.clone();
        new_base.insert(afflicted.clone(), (431, 2));
        assert!(verify_doll(&cosmetics, afflicted_choice, &before, &new_base).is_err());
        new_base.insert(afflicted_alias, (431, 2));
        verify_doll(&cosmetics, afflicted_choice, &before, &new_base)?;
        assert!(verify_doll(&cosmetics, citizen_choice, &before, &before).is_err());
        let mut removed_base = new_base.clone();
        removed_base.remove(&citizen);
        assert!(verify_doll(&cosmetics, afflicted_choice, &before, &removed_base).is_err());
        let mut wrong_look = existing_base.clone();
        wrong_look.insert(citizen.clone(), (128, 1));
        assert!(verify_doll(&cosmetics, citizen_choice, &before, &wrong_look).is_err());
        let mut wrong_mask = existing_base;
        wrong_mask.insert(citizen, (136, 3));
        assert!(verify_doll(&cosmetics, citizen_choice, &before, &wrong_mask).is_err());
        assert!(verify_doll(&cosmetics, citizen_choice, &before, &new_base).is_err());
        Ok(())
    }

    #[test]
    fn compact_bounds_and_joins() {
        assert_eq!(compact("a  b\nc"), "a-b-c");
        assert_eq!(compact(" "), "unknown");
        assert_eq!(compact(&"x".repeat(900)).len(), 400);
    }
}
