//! W09 exercises the real USE owner with two seeded Kooldown Aids and one Drome Cube.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_gameplay_items::concoctions::{ALL, Concoction};
use oteryn_gameplay_items::drome_cube::DROME_CUBE_KEY;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::presentation::{self, ActorCooldown, ActorCooldownKind};
use oteryn_protocol_oteryn::{tutorial_notices::TutorialNotice, world_object as wire};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

const RESET_NOTICE: &str = "Your spells are no longer on cooldown.";

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<NonZeroU32, u64>> {
    let mut quantities = BTreeMap::new();
    let inventory = client.inventory().ok_or("W09 inventory view missing")?;
    for entry in &inventory.entries {
        *quantities.entry(entry.item_definition_ref).or_default() += u64::from(entry.count.get());
    }
    Ok(quantities)
}

async fn use_item(
    client: &mut DevClientSession,
    reference: NonZeroU32,
    expected: (CommandStatus, wire::UseDisposition),
) -> TestResult {
    let payload = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("W09 USE encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await
        .map_err(|error| {
            verdict::context(
                format!("W09 USE reference={reference} expected={expected:?} observed={error}"),
                error,
            )
        })?;
    let (body, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("W09 USE refusal detail: {error:?}"))?;
    let disposition =
        wire::decode_use_result(body).map_err(|error| format!("W09 USE decode: {error:?}"))?;
    if (reply.status, disposition) != expected {
        return Err(format!(
            "W09 USE reference={reference} expected={expected:?} observed={:?}/{disposition:?}",
            reply.status
        )
        .into());
    }
    Ok(())
}

fn cube_reward(
    mut expected: BTreeMap<NonZeroU32, u64>,
    actual: &BTreeMap<NonZeroU32, u64>,
    pool: &[NonZeroU32],
) -> Option<NonZeroU32> {
    let reward = pool.iter().copied().find(|reference| {
        actual.get(reference).copied().unwrap_or_default()
            == expected.get(reference).copied().unwrap_or_default() + 1
    })?;
    *expected.entry(reward).or_default() += 1;
    (expected == *actual).then_some(reward)
}

async fn wait_inventory(
    client: &mut DevClientSession,
    expected: &BTreeMap<NonZeroU32, u64>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while quantities(client)? != *expected {
        if tokio::time::Instant::now() >= deadline {
            return Err("W09 first cast did not conjure exactly one wand".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

async fn cast_wand(client: &mut DevClientSession, spell: NonZeroU32) -> TestResult {
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("W09 caster vitals missing")?;
    let expected_mana = before
        .mana
        .checked_sub(250)
        .ok_or("W09 insufficient mana for Conjure Wand of Darkness")?;
    let cast = client.cast_spell(spell, SpellTarget::None, false).await?;
    if cast.status != CommandStatus::Accepted
        || cast.disposition != SpellCastDisposition::Cast
        || cast
            .actor_vitals_delta
            .as_ref()
            .map(|delta| delta.value.mana)
            != Some(expected_mana)
    {
        return Err(format!(
            "W09 Conjure Wand of Darkness did not spend 250 mana: before={before:?} cast={cast:?}"
        )
        .into());
    }
    Ok(())
}

async fn assert_conjure_cooldowns(
    client: &mut DevClientSession,
    spell: NonZeroU32,
    cast_started: tokio::time::Instant,
) -> TestResult<[ActorCooldown; 2]> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ActorCooldowns(frame) = event {
                let now = frame.value.server_now_ms;
                // Global: https://www.tibiawiki.com.br/wiki/Conjure_Wand_of_Darkness
                // The frame samples server time after commit. Both cooldowns share the cast time.
                let entries = frame.value.entries.as_slice();
                let [spell_entry, group_entry] = entries else {
                    return Err(format!(
                        "W09 cooldown entry count diverged sequence={} frame={:?}",
                        frame.server_sequence, frame.value
                    )
                    .into());
                };
                let elapsed_ms =
                    u64::try_from(cast_started.elapsed().as_millis())?.saturating_add(1);
                let group_remaining = group_entry.expires_at_ms.checked_sub(now);
                if (spell_entry.kind, spell_entry.id) != (ActorCooldownKind::Spell, spell.get())
                    || (group_entry.kind, group_entry.id) != (ActorCooldownKind::Group, 3)
                    || spell_entry
                        .expires_at_ms
                        .checked_sub(group_entry.expires_at_ms)
                        != Some(1_798_000)
                    || !group_remaining.is_some_and(|ms| {
                        ms > 0 && (2_000_u64.saturating_sub(elapsed_ms)..=2_000).contains(&ms)
                    })
                {
                    return Err(format!(
                        "W09 cooldown update diverged sequence={} frame={:?} cast_elapsed_ms={elapsed_ms}",
                        frame.server_sequence, frame.value
                    ).into());
                }
                evidence(&format!(
                    "W09_CHECK cooldown-update sequence={} spell_ms=1800000 support_ms=2000",
                    frame.server_sequence
                ));
                return Ok([*spell_entry, *group_entry]);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "W09 cooldown update missing through sequence={}",
                client.last_server_sequence()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn reconnect_cooldowns(
    client: DevClientSession,
    scope: &SeamClients<'_>,
    expected: [ActorCooldown; 2],
) -> TestResult<DevClientSession> {
    let session = client.join_snapshot().game_session_id;
    let sequence = client.last_server_sequence();
    let next_command = client.next_command_id();
    let generation = platform_generation(scope.descriptor, &scope.accounts[0]).await?;
    let (nonce, _) = fresh_grant_material(0xd2)?;
    let previous = client.disconnect();
    // Silent closes become reconnectable after the server liveness window.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
    while !matches!(
        session_loss_row(scope.url, *session.as_bytes()).await?,
        Some((1, _, _))
    ) {
        if tokio::time::Instant::now() >= deadline {
            return Err("W09 transport never became reconnectable".into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let credential = sign_recovery(
        scope.recovery,
        &scope.accounts[0],
        &scope.characters[0],
        &scope.world,
        generation,
        now_seconds()?,
        nonce,
    );
    let mut resumed = oteryn_dev_client::resume_session(
        oteryn_dev_client::RecoveryTransport {
            address: scope.address,
            server_name: "localhost",
            root_certificate: scope.certificate,
            deadline: Duration::from_secs(20),
        },
        previous,
        oteryn_dev_client::Recovery {
            reconnect_material: credential.as_bytes(),
            schema_revision: 1,
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
            appearance_items: None,
        },
    )
    .await?;
    let resumed_sequence = resumed.last_server_sequence();
    let resumed_next_command = resumed.next_command_id();
    let resumed_session = resumed.join_snapshot().game_session_id;
    let continuity = format!(
        "original_session={session:?} resumed_session={resumed_session:?} original_sequence={sequence} resumed_sequence={resumed_sequence} original_next_command={next_command} resumed_next_command={resumed_next_command}"
    );
    let cooldowns = resumed
        .session_mut()
        .gated_snapshot(presentation::STATE_DOMAIN_ACTOR_COOLDOWNS)
        .ok_or("W09 reconnect omitted ACTOR_COOLDOWNS")
        .and_then(|snapshot| {
            presentation::decode_actor_cooldowns(&snapshot.payload)
                .map_err(|_| "W09 reconnect sent malformed ACTOR_COOLDOWNS")
        })
        .map_err(|error| format!("{error} {continuity}"))?;
    let active: Vec<_> = expected
        .into_iter()
        .filter(|entry| entry.expires_at_ms > cooldowns.server_now_ms)
        .collect();
    if resumed_session != session
        || resumed_sequence <= sequence
        || resumed_next_command != next_command
        || !active
            .iter()
            .any(|entry| entry.kind == ActorCooldownKind::Spell)
        || cooldowns.entries != active
    {
        return Err(format!(
            "W09 reconnect cooldown replay diverged {continuity} frame={cooldowns:?} expected={active:?}"
        ).into());
    }
    evidence(&format!(
        "W09_CHECK cooldown-reconnect {continuity} spell-deadline=preserved"
    ));
    Ok(resumed)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let cube_id = character_prep::item_id(DROME_CUBE_KEY)?;
    let aid_id = Concoction::KooldownAid.item_id();
    let index = provisioning::item_index()?;
    let cube = provisioning::item_reference(&index, cube_id)?;
    let aid = provisioning::item_reference(&index, aid_id)?;
    let pool = ALL
        .iter()
        .map(|kind| provisioning::item_reference(&index, kind.item_id()))
        .collect::<TestResult<Vec<_>>>()?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W09_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let mut client = connect_character(&scope, 0xd2, 0).await?;
    let mut expected = quantities(&client)?;
    if expected.contains_key(&cube) || expected.contains_key(&aid) {
        return Err("W09 requires a fresh dedicated character".into());
    }
    let preparation_session = client.join_snapshot().game_session_id;
    character_prep::declare_seeded(
        "W09.full",
        "level-41 Premium sorcerer, one Drome Cube and two Kooldown Aids",
        "ordinary leveling exceeds the row budget and the inputs have no authored acquisition route",
    );
    request_fixture_control(
        uuid_text(preparation_session.as_bytes()),
        "--profile",
        "w09-kooldown".into(),
    )
    .await?;
    use_item(
        &mut client,
        aid,
        (CommandStatus::Accepted, wire::UseDisposition::NothingToUse),
    )
    .await?;
    if quantities(&client)? != expected {
        return Err("W09 missing Aid refusal changed inventory".into());
    }
    character_prep::grant_item(&client, cube_id, 1).await?;
    character_prep::grant_item(&client, aid_id, 2).await?;
    logout(&mut client).await?;
    let mut client = connect_character(&scope, 0xd2, 0).await?;
    if client.join_snapshot().game_session_id == preparation_session {
        return Err("W09 preparation reused its original session".into());
    }
    evidence("FIXTURE row=W09.full input=premium");
    character_prep::grant_premium(&client).await?;
    let level = full_e2ef7::player_level(&mut client).await?;
    let vitals = client.actor_vitals().ok_or("W09 seeded vitals missing")?;
    if (level, vitals.max_health, vitals.max_mana) != (41, 350, 1080) {
        return Err(format!(
            "W09 expected level-41 vitals 350/1080: level={level} vitals={vitals:?}"
        )
        .into());
    }
    expected.extend([(cube, 1), (aid, 2)]);
    if quantities(&client)? != expected {
        return Err("W09 fresh admission did not serve exactly the seeded items".into());
    }

    let committed = (CommandStatus::Accepted, wire::UseDisposition::Committed);
    let missing = (CommandStatus::Accepted, wire::UseDisposition::NothingToUse);
    let book = native_spell_book()?;
    let spell = crate::qualification::client_spell_journey::spell_index(book, "exevo gran mort")?;
    let natural_cooldown = Duration::from_micros(
        book.indexed(spell)
            .ok_or("W09 Conjure Wand of Darkness missing from native book")?
            .cooldown_micros,
    );
    // Global: https://www.tibiawiki.com.br/wiki/Conjure_Wand_of_Darkness
    if natural_cooldown != Duration::from_secs(1800) {
        return Err(format!("W09 unexpected donor spell cooldown: {natural_cooldown:?}").into());
    }
    let wand = provisioning::item_reference(&index, 25760)?;
    if expected.contains_key(&wand)
        || !client
            .inventory()
            .is_some_and(|inventory| inventory.main_backpack.is_some())
    {
        return Err("W09 requires a main backpack with no Wand of Darkness".into());
    }
    client.take_events();
    let first_cast = tokio::time::Instant::now();
    cast_wand(&mut client, spell).await?;
    *expected.entry(wand).or_default() += 1;
    let cooldowns = assert_conjure_cooldowns(&mut client, spell, first_cast).await?;
    wait_inventory(&mut client, &expected).await?;
    let mut client = reconnect_cooldowns(client, &scope, cooldowns).await?;
    if quantities(&client)? != expected {
        return Err("W09 reconnect changed the first conjure inventory".into());
    }
    evidence(
        "GAP row=W09.full status=BLOCKED-CONTRACT check=spell-and-group-cooldowns-survive-full-logout",
    );
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("W09 caster vitals missing")?;
    let blocked = client.cast_spell(spell, SpellTarget::None, false).await?;
    if blocked.status != CommandStatus::Accepted
        || blocked.disposition != SpellCastDisposition::CoolingDown
        || blocked.actor_vitals_delta.is_some()
        || client.actor_vitals().copied() != Some(before)
        || quantities(&client)? != expected
    {
        return Err(format!(
            "W09 Conjure Wand of Darkness pre-state was not a resource-free cooldown refusal: {blocked:?}"
        )
        .into());
    }
    client.take_events();
    use_item(&mut client, aid, committed).await?;
    cast_wand(&mut client, spell).await?;
    if first_cast.elapsed() >= natural_cooldown {
        return Err(
            "W09 Conjure Wand of Darkness recast did not precede natural cooldown expiry".into(),
        );
    }
    expected.insert(aid, 1);
    *expected.entry(wand).or_default() += 1;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut saw_reset = false;
    loop {
        saw_reset |= client.take_events().iter().any(|event| {
            matches!(event, SessionEvent::TutorialNotice(delta)
                if matches!(&delta.value, TutorialNotice::StatusText(text) if text == RESET_NOTICE))
        });
        if saw_reset && quantities(&client)? == expected {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W09 Kooldown Aid did not emit the reset notice and exact debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    use_item(&mut client, cube, committed).await?;
    expected.remove(&cube);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (reward, settled) = loop {
        let actual = quantities(&client)?;
        if let Some(reward) = cube_reward(expected.clone(), &actual, &pool) {
            break (reward, actual);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "W09 Cube did not debit exactly one unit and grant exactly one donor reward".into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    evidence(&format!(
        "W09_CHECK reset-notice=observed cube-reward-ref={reward}"
    ));
    let used_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    let mut fresh = connect_character(&scope, 0xd2, 0).await?;
    if fresh.join_snapshot().game_session_id == used_session {
        return Err("W09 durable verification reused the USE session".into());
    }
    if quantities(&fresh)? != settled {
        return Err("W09 fresh admission did not serve the settled inventory".into());
    }
    use_item(&mut fresh, cube, missing).await?;
    if quantities(&fresh)? != settled {
        return Err("W09 missing Cube refusal changed inventory".into());
    }
    use_item(
        &mut fresh,
        aid,
        (CommandStatus::Accepted, wire::UseDisposition::Exhausted),
    )
    .await?;
    if quantities(&fresh)? != settled {
        return Err("W09 refused Kooldown Aid consumed the second unit".into());
    }
    let refused_session = fresh.join_snapshot().game_session_id;
    logout(&mut fresh).await?;
    let mut verified = connect_character(&scope, 0xd2, 0).await?;
    if verified.join_snapshot().game_session_id == refused_session
        || quantities(&verified)? != settled
    {
        return Err("W09 fresh admission did not preserve inventory through both refusals".into());
    }
    let sequence = verified.last_server_sequence();
    let cooldowns = verified
        .session_mut()
        .gated_snapshot(presentation::STATE_DOMAIN_ACTOR_COOLDOWNS)
        .ok_or("W09 full relog omitted ACTOR_COOLDOWNS")
        .and_then(|snapshot| {
            presentation::decode_actor_cooldowns(&snapshot.payload)
                .map_err(|_| "W09 full relog sent malformed ACTOR_COOLDOWNS")
        })?;
    if !cooldowns.entries.iter().any(|entry| {
        entry.kind == ActorCooldownKind::Spell
            && entry.id == spell.get()
            && entry.expires_at_ms > cooldowns.server_now_ms
    }) {
        return Err(format!(
            "W09 BLOCKED-CONTRACT full relog lost the active spell cooldown sequence={sequence} frame={cooldowns:?}"
        ).into());
    }
    logout(&mut verified).await?;
    evidence(
        "W09_CHECK spell-reset=paid-recast durable-inventory=verified cooldown-refusal=observed",
    );
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W09 logout refused: {reply:?}").into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "reset-and-cube-transform".to_owned()),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W09.full status={status} seconds={:.3} evidence={}",
        started.elapsed().as_secs_f64(),
        detail.split_whitespace().collect::<Vec<_>>().join("-")
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_reward_requires_one_pool_unit_and_preserves_every_other_stack() -> TestResult {
        let reward = NonZeroU32::new(1).ok_or("reward ref")?;
        let unrelated = NonZeroU32::new(2).ok_or("unrelated ref")?;
        let before = BTreeMap::from([(reward, 2), (unrelated, 1)]);
        let after = BTreeMap::from([(reward, 3), (unrelated, 1)]);
        assert_eq!(cube_reward(before.clone(), &after, &[reward]), Some(reward));
        assert_eq!(cube_reward(before.clone(), &before, &[reward]), None);
        assert_eq!(cube_reward(before.clone(), &after, &[unrelated]), None);
        assert_eq!(
            cube_reward(before, &BTreeMap::from([(reward, 3)]), &[reward]),
            None
        );
        Ok(())
    }
}
