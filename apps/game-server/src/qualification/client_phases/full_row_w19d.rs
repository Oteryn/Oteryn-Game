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
const NEEDED: &str = "needed:outfit-or-mount-item-6579-or-12551-in-inventory;no-npc-seller(content/services/trade);drops-only-from-high-level-creatures(content/loot)";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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
) -> BTreeMap<String, (u32, u8)> {
    let facts: Vec<_> = grants
        .iter()
        .map(|key| (key.clone(), item_cosmetics::PROVENANCE.to_owned()))
        .collect();
    let earned = EarnedCosmetics {
        content,
        facts: &facts,
    }
    .with_sex(Sex::Female);
    let mut expected = before.clone();
    for key in content.outfit_keys() {
        let key = key.to_owned();
        let prior = before.get(&key);
        if prior.is_none() && earned.outfit(&key) != UnlockFact::Compatible {
            continue;
        }
        let earned_addons = u8::from(earned.addon(&key, 1) == UnlockFact::Compatible)
            | (u8::from(earned.addon(&key, 2) == UnlockFact::Compatible) << 1);
        if let Some(look) = content.look_type(&key, Sex::Female) {
            expected.insert(
                key,
                (look, prior.map_or(0, |value| value.1) | earned_addons),
            );
        }
    }
    expected
}

fn verify_doll(
    content: &NativeCosmeticsView,
    before: &BTreeMap<String, (u32, u8)>,
    after: &BTreeMap<String, (u32, u8)>,
) -> TestResult {
    let rule =
        item_cosmetics::rule("oteryn:item.tibia.i6579").ok_or("step=doll observed=no-rule")?;
    if before == after {
        return Err("step=doll observed=no-outfit-change".into());
    }
    let choice = rule
        .choices
        .iter()
        .find(|choice| outfit_grant_projection(content, before, &choice.grants) == *after)
        .ok_or("step=doll observed=outfits-do-not-match-a-single-authored-draw")?;
    evidence(&format!(
        "W19D_DOLL_ASSERT grant={} consumed=1",
        choice.owned
    ));
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let cosmetics = cosmetics_fixture::decode_cosmetics(&native_input()?.cosmetics.bytes)?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W19D_ACCOUNT").await?;
    let clients = dedicated.clients(clients);
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
    let mut funded = initial_inventory.clone();
    for id in AFFLICTED_PIECES.into_iter().chain([ADDON_DOLL]) {
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
    }
    let afflicted_reference = reference(AFFLICTED_OUTFIT_PIECE)?;
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
    expect_item_use(
        &mut client,
        afflicted_reference,
        use_wire::UseDisposition::Committed,
    )
    .await?;
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
    verify_doll(&cosmetics, &afflicted, &after_doll)?;
    if appearance_snapshot(&mut client)?.catalogue.mounts != initial_mounts {
        return Err("step=outfit-use observed=unexpected-mount-grant".into());
    }
    let mut duplicate_inputs = initial_inventory.clone();
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
    expect_item_use(
        &mut client,
        mount_reference,
        use_wire::UseDisposition::Committed,
    )
    .await?;
    let mut client = relog(client, &clients).await?;
    let mut observed_mounts = appearance_snapshot(&mut client)?.catalogue.mounts;
    observed_mounts.sort();
    if quantities(&client)? != duplicate_inputs
        || outfits(&mut client)? != after_doll
        || observed_mounts != mounted
    {
        return Err("step=mount observed=unexpected-persisted-outcome".into());
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
        let existing_base = BTreeMap::from([
            (citizen.clone(), (136, 1)),
            (citizen_alias.clone(), (136, 0)),
        ]);
        verify_doll(&cosmetics, &before, &existing_base)?;
        let mut unearned_alias = existing_base.clone();
        unearned_alias.insert(citizen_alias, (136, 1));
        assert!(verify_doll(&cosmetics, &before, &unearned_alias).is_err());
        let mut new_base = before.clone();
        new_base.insert(afflicted.clone(), (431, 2));
        assert!(verify_doll(&cosmetics, &before, &new_base).is_err());
        new_base.insert(afflicted_alias, (431, 2));
        verify_doll(&cosmetics, &before, &new_base)?;
        assert!(verify_doll(&cosmetics, &before, &before).is_err());
        let mut removed_base = new_base.clone();
        removed_base.remove(&citizen);
        assert!(verify_doll(&cosmetics, &before, &removed_base).is_err());
        let mut wrong_look = existing_base.clone();
        wrong_look.insert(citizen.clone(), (128, 1));
        assert!(verify_doll(&cosmetics, &before, &wrong_look).is_err());
        let mut wrong_mask = existing_base;
        wrong_mask.insert(citizen, (136, 3));
        assert!(verify_doll(&cosmetics, &before, &wrong_mask).is_err());
        Ok(())
    }

    #[test]
    fn compact_bounds_and_joins() {
        assert_eq!(compact("a  b\nc"), "a-b-c");
        assert_eq!(compact(" "), "unknown");
        assert_eq!(compact(&"x".repeat(900)).len(), 400);
    }
}
