//! W09 exercises the real USE owner with two seeded Kooldown Aids and one Drome Cube.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_gameplay_items::concoctions::Concoction;
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

async fn expected_cube_draws(
    scope: &SeamClients<'_>,
    pool: &[NonZeroU32],
) -> TestResult<Vec<NonZeroU32>> {
    use sha2::{Digest, Sha256};
    use sqlx::Connection;

    // Read only the identity of the fixture input. All outcome checks use the live inventory.
    let mut connection = sqlx::PgConnection::connect(scope.url).await?;
    let stacks: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT uuid_send(i.item_instance_id), i.quantity::bigint
         FROM game_item_container_entries e
         JOIN game_item_container_slots b USING (character_id, world_id)
         JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id AND i.world_id=e.world_id
         WHERE e.character_id=encode($1,'hex')::uuid
           AND e.parent_item_instance_id=b.item_instance_id
           AND i.definition_production_key=$2 AND i.lifecycle=1
         ORDER BY e.placement_ordinal DESC
         LIMIT 5",
    )
    .bind(scope.characters[0].as_slice())
    .bind(DROME_CUBE_KEY)
    .fetch_all(&mut connection)
    .await?;
    connection.close().await?;
    if stacks.len() != 4 || stacks.iter().any(|(_, quantity)| *quantity != 1) {
        return Err(format!(
            "W09 fixture requires four single-unit Cubes in the main backpack; observed quantities {:?}",
            stacks.iter().map(|(_, quantity)| quantity).collect::<Vec<_>>()
        ).into());
    }
    let draws = stacks
        .iter()
        .map(|(source, quantity)| {
            // Independently calculate the source-unit occurrence in hotkey display order.
            let mut occurrence = b"oteryn.drome-cube.reward.v1".to_vec();
            occurrence.extend_from_slice(source);
            occurrence.extend_from_slice(&u64::try_from(*quantity)?.to_be_bytes());
            let sample = Sha256::digest(&occurrence)
                .into_iter()
                .find(|byte| *byte < 240)
                .ok_or("W09 Cube fixture draw exhausted entropy")?;
            Ok(pool[usize::from(sample % 20)])
        })
        .collect::<TestResult<Vec<_>>>()?;
    if draws.windows(2).all(|pair| pair[0] == pair[1]) {
        return Err("W09 Cube fixture draws do not distinguish a fixed reward".into());
    }
    Ok(draws)
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

async fn cast_paid(client: &mut DevClientSession, spell: NonZeroU32, mana_cost: u32) -> TestResult {
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("W09 caster vitals missing")?;
    let expected_mana = before
        .mana
        .checked_sub(mana_cost)
        .ok_or("W09 insufficient mana for cooldown probe")?;
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
            "W09 spell {spell} did not spend {mana_cost} mana: before={before:?} cast={cast:?}"
        )
        .into());
    }
    Ok(())
}

async fn assert_cast_cooldowns(
    client: &mut DevClientSession,
    spell: NonZeroU32,
    cast_started: tokio::time::Instant,
    spell_ms: u64,
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
                        != spell_ms.checked_sub(2_000)
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
                    "W09_CHECK cooldown-update sequence={} spell_ms={spell_ms} support_ms=2000",
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

async fn assert_cooling_down(
    client: &mut DevClientSession,
    spell: NonZeroU32,
    expected: &BTreeMap<NonZeroU32, u64>,
) -> TestResult {
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("W09 caster vitals missing")?;
    let blocked = client.cast_spell(spell, SpellTarget::None, false).await?;
    if blocked.status != CommandStatus::Accepted
        || blocked.disposition != SpellCastDisposition::CoolingDown
        || blocked.actor_vitals_delta.is_some()
        || client.actor_vitals().copied() != Some(before)
        || &quantities(client)? != expected
    {
        return Err(format!(
            "W09 Conjure Wand of Darkness was not a resource-free cooldown refusal: {blocked:?}"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let cube_id = character_prep::item_id(DROME_CUBE_KEY)?;
    let aid_id = Concoction::KooldownAid.item_id();
    let index = provisioning::item_index()?;
    let cube = provisioning::item_reference(&index, cube_id)?;
    let aid = provisioning::item_reference(&index, aid_id)?;
    // Global pool: https://www.tibiawiki.com.br/wiki/Drome_Cube.
    // Item IDs for those twenty names: Crystal 201189f8, data/scripts/actions/items/drome_cube.lua.
    let pool = (36723..=36742)
        .map(|id| provisioning::item_reference(&index, id))
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
        "level-41 Premium sorcerer, four Drome Cubes and two Kooldown Aids",
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
    character_prep::grant_item(&client, cube_id, 4).await?;
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
    expected.extend([(cube, 4), (aid, 2)]);
    if quantities(&client)? != expected {
        return Err("W09 fresh admission did not serve exactly the seeded items".into());
    }

    let expected_draws = expected_cube_draws(&scope, &pool).await?;
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
    cast_paid(&mut client, spell, 250).await?;
    *expected.entry(wand).or_default() += 1;
    let cooldowns = assert_cast_cooldowns(&mut client, spell, first_cast, 1_800_000).await?;
    wait_inventory(&mut client, &expected).await?;
    let mut client = reconnect_cooldowns(client, &scope, cooldowns).await?;
    if quantities(&client)? != expected {
        return Err("W09 reconnect changed the first conjure inventory".into());
    }
    assert_cooling_down(&mut client, spell, &expected).await?;
    // Global: https://www.tibiawiki.com.br/wiki/Magic_Shield
    // Arm a second spell and a fresh Support deadline immediately before using the Aid.
    let shield = crate::qualification::client_spell_journey::spell_index(book, "utamo vita")?;
    client.take_events();
    let shield_cast = tokio::time::Instant::now();
    cast_paid(&mut client, shield, 50).await?;
    let shield_cooldowns = assert_cast_cooldowns(&mut client, shield, shield_cast, 14_000).await?;
    let mut reset_prestate = [cooldowns[0], shield_cooldowns[0], shield_cooldowns[1]];
    reset_prestate.sort_by_key(|entry| (entry.kind, entry.id));
    let used_at = tokio::time::Instant::now();
    use_item(&mut client, aid, committed).await?;
    let committed_at = tokio::time::Instant::now();
    let reset_until = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut saw_reset = false;
    let mut saw_clear = false;
    while !saw_reset || !saw_clear {
        for event in client.take_events() {
            match event {
                SessionEvent::ActorCooldowns(frame) => {
                    let expected_clear = reset_prestate
                        .iter()
                        .map(|entry| ActorCooldown {
                            expires_at_ms: 0,
                            ..*entry
                        })
                        .collect::<Vec<_>>();
                    if reset_prestate
                        .iter()
                        .any(|entry| entry.expires_at_ms <= frame.value.server_now_ms)
                        || frame.value.entries != expected_clear
                    {
                        return Err(format!("W09 Aid clearance mismatch sequence={} expected={expected_clear:?} frame={:?}", frame.server_sequence, frame.value).into());
                    }
                    saw_clear = true;
                }
                SessionEvent::TutorialNotice(delta)
                    if matches!(&delta.value,
                    TutorialNotice::StatusText(text) if text == RESET_NOTICE) =>
                {
                    saw_reset = true
                }
                _ => {}
            }
        }
        if saw_reset && saw_clear {
            break;
        }
        if tokio::time::Instant::now() >= reset_until {
            return Err(format!(
                "W09 missing Aid clearance or notice after sequence={}",
                client.last_server_sequence()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let second_cast = tokio::time::Instant::now();
    cast_paid(&mut client, spell, 250).await?;
    let durable_cooldowns =
        assert_cast_cooldowns(&mut client, spell, second_cast, 1_800_000).await?;
    if first_cast.elapsed() >= natural_cooldown {
        return Err(
            "W09 Conjure Wand of Darkness recast did not precede natural cooldown expiry".into(),
        );
    }
    expected.insert(aid, 1);
    *expected.entry(wand).or_default() += 1;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if quantities(&client)? == expected {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W09 Kooldown Aid did not emit the reset notice and exact debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    for (offset, reward) in expected_draws.into_iter().enumerate() {
        use_item(&mut client, cube, committed).await?;
        let remaining = 3 - u64::try_from(offset)?;
        if remaining == 0 {
            expected.remove(&cube);
        } else {
            expected.insert(cube, remaining);
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let actual = quantities(&client)?;
            if cube_reward(expected.clone(), &actual, &[reward]) == Some(reward) {
                expected = actual;
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "W09 Cube draw {offset} expected one reward ref={reward} with {remaining} Cubes left"
                ).into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
        evidence(&format!(
            "W09_CHECK cube-draw={offset} reward-ref={reward} remaining={remaining}"
        ));
    }
    let settled = expected;
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
    fresh.take_events();
    let minimum_age = committed_at.elapsed().as_secs();
    let upper = 86_400_u64.saturating_sub(minimum_age);
    use_item(
        &mut fresh,
        aid,
        (CommandStatus::Accepted, wire::UseDisposition::Exhausted),
    )
    .await?;
    if quantities(&fresh)? != settled {
        return Err("W09 refused Kooldown Aid consumed the second unit".into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let events = fresh.take_events();
        let remaining = events.iter().find_map(|event| {
            let SessionEvent::TutorialNotice(delta) = event else {
                return None;
            };
            let TutorialNotice::StatusText(text) = &delta.value else {
                return None;
            };
            text.strip_prefix("Kooldown Aid can be used again in ")?
                .strip_suffix(" seconds.")?
                .parse::<u64>()
                .ok()
        });
        if let Some(seconds) = remaining {
            let lower = 86_400_u64.saturating_sub(used_at.elapsed().as_secs().saturating_add(1));
            if upper == 86_400 || !(lower..=upper).contains(&seconds) {
                return Err(
                    format!("W09 Aid remaining seconds {seconds} outside elapsed-time bounds {lower}..={upper}").into(),
                );
            }
            evidence(&format!(
                "W09_CHECK cooldown-aid-reuse remaining_seconds={seconds}"
            ));
            break;
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!(
                "W09 missing remaining-time notice after sequence={}",
                fresh.last_server_sequence()
            )
            .into());
        }
        fresh.service_liveness(Duration::from_millis(50)).await?;
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
    let active: Vec<_> = durable_cooldowns
        .into_iter()
        .filter(|entry| entry.expires_at_ms > cooldowns.server_now_ms)
        .collect();
    if !active
        .iter()
        .any(|entry| entry.kind == ActorCooldownKind::Spell)
        || cooldowns.entries != active
    {
        return Err(format!(
            "W09 full relog changed the active deadlines sequence={sequence} expected={active:?} frame={cooldowns:?}"
        ).into());
    }
    evidence("FIXTURE row=W09.full input=premium purpose=post-relog-cast");
    character_prep::grant_premium(&verified).await?;
    assert_cooling_down(&mut verified, spell, &settled).await?;
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
