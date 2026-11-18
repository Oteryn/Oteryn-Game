//! W02 player scenario, derived from the matrix/re-audit row and the
//! `boss_activity_wire` owner:
//! 1. Query the admitted character's boss cooldowns; expect an accepted, bounded
//!    native reply with no Faceless Bane cooldown for a fresh character.
//! 2. Walk the compiled world toward an authored Faceless Bane lever cell and send
//!    `BOSS_LEVER(oteryn:boss.faceless_bane)`; expect either an accepted activity
//!    reply plus the Instance scope/position deltas, or a typed refusal identifying
//!    the first unmet player prerequisite (position, level 250, party count,
//!    cooldown, capacity, or content).
//! 3. In an accepted fresh Instance, find the served boss entity, attack it through
//!    native combat until its removal/death frames, and require return to Channel.
//! 4. Query cooldowns and the personal reward chest; require the new boss deadline,
//!    participation kill progress, and a nonempty personal reward group.
//! 5. Send the lever command again while its deadline is active; require the typed
//!    `Cooldown` refusal and no second Instance handoff.
//! 6. Log out normally, obtain a fresh admission, and query cooldown/reward state
//!    again; require the same durable deadline and personal reward group.
//!
//! This journey always performs steps 1 and 2. If the ordinary node-boot character
//! cannot legitimately reach/qualify for the lever, its runtime refusal is the row
//! verdict and the later dependent steps are reported as unproven rather than
//! fabricated.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::boss_encounter as wire;

pub(super) const BOSS: &str = "oteryn:boss.faceless_bane";
const LEVERS: &[TilePos] = &[
    TilePos {
        x: 33638,
        y: 32562,
        floor: -13,
    },
    TilePos {
        x: 33639,
        y: 32562,
        floor: -13,
    },
    TilePos {
        x: 33640,
        y: 32562,
        floor: -13,
    },
    TilePos {
        x: 33641,
        y: 32562,
        floor: -13,
    },
    TilePos {
        x: 33642,
        y: 32562,
        floor: -13,
    },
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(320), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=W02.full status=PASS seconds={:.3} evidence=lever-instance-defeat-cooldown-participation-reward-fresh-admission",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W02.full status=FAIL seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            evidence_token(&error.to_string())
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=W02.full status=FAIL seconds={:.3} evidence=step-timeout:320s;need=completed-native-boss-lifecycle",
            started.elapsed().as_secs_f64()
        )),
    }
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:=,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

async fn command(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(wire::CAPABILITY_BOSS_ENCOUNTER_V1, kind, payload)
        .await?)
}

async fn cooldowns(client: &mut DevClientSession) -> TestResult<Vec<u8>> {
    let reply = command(client, wire::COMMAND_TYPE_BOSS_COOLDOWNS, &[]).await?;
    if reply.status != CommandStatus::Accepted || reply.payload.is_empty() {
        return Err(
            format!("step-1-cooldown-query:observed={reply:?};need=accepted-native-list").into(),
        );
    }
    Ok(reply.payload)
}

async fn walk_to_boss_lever(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    // Route from the live player cell to the authored encounter lever, servicing the
    // connection during the search for entrances outside the starting town.
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = reach_route(client, world, from, |at| LEVERS.contains(&at))
        .await?
        .map_err(|failure| format!("step-2-lever-walk:observed=no-route-from-{from:?} {failure};need=walk-to-authored-lever"))?;
    walk(client, world, &path).await
}

pub(super) async fn fight_boss(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    row: &str,
) -> TestResult<[u8; 16]> {
    use oteryn_protocol_oteryn::activity_scope::ActivityScope;
    walk_to_boss_lever(client, clients).await?;
    let lever_position = client.world_spatial().actor_position;
    let channel_session = client.join_snapshot().game_session_id;

    let payload =
        wire::encode_boss_key(BOSS).map_err(|error| format!("step-2-lever-encode:{error:?}"))?;
    let lever = command(client, wire::COMMAND_TYPE_BOSS_LEVER, &payload).await?;
    let activity = wire::decode_boss_activity_reply(&lever.payload).map_err(|error| {
        format!(
            "step-2-lever-reply:status={:?};decode={error:?}",
            lever.status
        )
    })?;
    let handoff = match activity {
        wire::BossActivityReply::Refused(reason) => {
            // This is a real server decision after the native command, not a
            // source-derived verdict. Preserve the exact typed refusal in evidence.
            let _ = client.session_mut().logout().await;
            return Err(format!(
                "step-2-lever-entry:observed-status={:?},refusal={reason:?};need=reachable-authored-lever-and-legitimate-level-250-party",
                lever.status
            ).into());
        }
        wire::BossActivityReply::Accepted { handoff, .. }
            if lever.status == CommandStatus::Accepted =>
        {
            let instance_session = client.join_snapshot().game_session_id;
            let instance_position = client.world_spatial().actor_position;
            let linked_instance = client.session_mut().activity_scope().is_some_and(|scope| {
                scope.handoff == handoff
                    && scope.source_session == *channel_session.as_bytes()
                    && scope.session == *instance_session.as_bytes()
                    && matches!(scope.scope, ActivityScope::Instance { world, .. } if world == clients.world)
            });
            if !linked_instance
                || instance_session == channel_session
                || instance_position == lever_position
            {
                return Err(format!(
                    "step-2-instance-deltas:observed-reply=Accepted-handoff-{handoff:?},session-changed={},position-changed={};need=Instance-scope-and-position-deltas",
                    instance_session != channel_session,
                    instance_position != lever_position
                )
                .into());
            }
            evidence(&format!(
                "{row}_CHECK step=boss-entry status=PASS observed=accepted-instance-session-and-position-deltas"
            ));
            handoff
        }
        other => {
            return Err(format!("step-2-lever-entry:observed-status={:?},reply={other:?};need=accepted-consistent-handoff", lever.status).into());
        }
    };
    let instance_session = client.join_snapshot().game_session_id;

    // The accepted branch must observe a real boss before combat. It deliberately
    // fails with the served frames when the handoff did not publish the entity;
    // it never substitutes a defeated/timeout control command for a player kill.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let target = loop {
        if let Some(entity) = named_creature(client, "faceless bane").await? {
            break entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-3-boss-combat:observed=accepted-handoff-without-creature-frame;need=served-instance-boss-entity".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(
            format!("step-3-boss-combat:observed={attack:?};need=accepted-native-attack").into(),
        );
    }
    let defeat_deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    verdict::Harness::global().bounded_wait(defeat_deadline, "bounded-observation");
    while client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|entity| entity.entity == target))
    {
        if tokio::time::Instant::now() >= defeat_deadline {
            client.session_mut().attack_target(None).await?;
            return Err("step-3-boss-combat:observed=attack-accepted-boss-still-present-after-150s;need=legitimately-earned-combat-strength-or-longer-budget".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    client.session_mut().attack_target(None).await?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let (x, y, floor, wait_ms): (i32, i32, i16, i64) = sqlx::query_as(
        "SELECT q.exit_x, q.exit_y, q.exit_floor, b.after_kill_ms
         FROM game_activity_runtime_placements q
         JOIN game_activity_handoff_participants p USING (handoff_id, character_id)
         JOIN game_boss_room_activities b ON b.admission = q.handoff_id
         WHERE q.handoff_id = encode($1,'hex')::uuid
           AND p.destination_game_session_id = encode($2,'hex')::uuid",
    )
    .bind(handoff.as_slice())
    .bind(instance_session.as_bytes().as_slice())
    .fetch_one(&mut database)
    .await?;
    let (return_wait, expected_exit) = return_expectation(
        ActorPosition { x, y, floor },
        Duration::from_millis(wait_ms.try_into()?),
    );
    let return_deadline = tokio::time::Instant::now() + return_wait;
    verdict::Harness::global().bounded_wait(return_deadline, "boss-return");
    while client.join_snapshot().game_session_id == instance_session
        || client.world_spatial().actor_position != expected_exit
    {
        if tokio::time::Instant::now() >= return_deadline {
            return Err(format!(
                "step-3-return-to-channel:observed-session-changed={},position={:?};need=new-Channel-session-at-{expected_exit:?}",
                client.join_snapshot().game_session_id != instance_session,
                client.world_spatial().actor_position
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let returned = client.session_mut().activity_scope().is_some_and(|scope| {
        scope.source_session == *instance_session.as_bytes()
            && scope.scope
                == (ActivityScope::Channel {
                    world: clients.world,
                    channel: clients.channel,
                })
    });
    if !returned {
        return Err("step-3-return-to-channel:observed=missing-matching-channel-scope".into());
    }
    evidence(&format!(
        "{row}_CHECK step=boss-return status=PASS observed=native-boss-removal-and-channel-return"
    ));

    Ok(handoff)
}

fn return_expectation(exit: ActorPosition, after_kill: Duration) -> (Duration, ActorPosition) {
    (after_kill.saturating_add(Duration::from_secs(20)), exit)
}

pub(super) async fn named_creature(
    client: &mut DevClientSession,
    name: &str,
) -> TestResult<Option<oteryn_dev_client::EntityRef>> {
    let candidates: Vec<_> = client
        .world_entities()
        .ok_or("missing creature entity view")?
        .iter()
        .filter(|entry| entry.kind == oteryn_dev_client::EntityKind::Creature)
        .map(|entry| entry.entity)
        .collect();
    for target in candidates {
        use oteryn_protocol_oteryn::item_look::ItemLookResult;
        let look = actor_look(client, target).await?;
        if matches!(look, ItemLookResult::Text(_)) && classify_actor_name(look, name)? {
            return Ok(Some(target));
        }
    }
    Ok(None)
}

pub(super) async fn actor_look(
    client: &mut DevClientSession,
    target: oteryn_dev_client::EntityRef,
) -> TestResult<oteryn_protocol_oteryn::item_look::ItemLookResult> {
    use oteryn_protocol_oteryn::item_look as look;
    let selection = look::ItemLookSelection { extended: true };
    let payload = look::encode_item_look_intent_for(
        look::ItemLookIntent {
            target: look::ItemLookTarget::Creature(target),
        },
        selection,
    )
    .map_err(|error| format!("creature LOOK encode: {error:?}"))?;
    let reply = client.session_mut().gated_command(28, 30, &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("creature LOOK refused: {reply:?}").into());
    }
    look::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("actor LOOK decode: {error:?}").into())
}

pub(super) fn classify_actor_name(
    result: oteryn_protocol_oteryn::item_look::ItemLookResult,
    name: &str,
) -> TestResult<bool> {
    use oteryn_protocol_oteryn::item_look::ItemLookResult;
    let text = match result {
        ItemLookResult::Text(text) => text,
        ItemLookResult::Refused(reason) => {
            return Err(format!("actor LOOK identity indeterminate: {reason:?}").into());
        }
    };
    if let Some(observed) = text
        .strip_prefix("You see ")
        .and_then(|text| text.strip_suffix('.'))
    {
        let observed = ["the ", "an ", "a "]
            .iter()
            .find_map(|article| observed.strip_prefix(article))
            .unwrap_or(observed);
        return Ok(observed.eq_ignore_ascii_case(name));
    }
    Err("actor LOOK did not return a recognizable actor description".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb2).await?;
    let before = cooldowns(&mut client).await?;
    if contains_key(&before, BOSS) {
        return Err(
            "step-1-fresh-cooldown:observed=faceless-bane-present;need=fresh-node-boot-character"
                .into(),
        );
    }
    evidence("W02_CHECK step=1 status=PASS observed=accepted-empty-target-cooldown");

    fight_boss(&mut client, clients, "W02").await?;

    let after = cooldowns(&mut client).await?;
    let settled_cooldown = nested_entry(&after, 2, BOSS)?
        .ok_or("step-4-cooldown:observed=no-target-entry;need=new-durable-boss-deadline")?;
    let observed_at = varint_field(&after, 1)?;
    let cooldown_deadline = varint_field(&settled_cooldown, 2)?;
    if observed_at
        .zip(cooldown_deadline)
        .is_none_or(|(now, deadline)| deadline <= now)
    {
        return Err(format!(
            "step-4-cooldown:observed-now={observed_at:?},deadline={cooldown_deadline:?};need=future-boss-deadline"
        )
        .into());
    }
    let progress = nested_entry(&after, 4, BOSS)?
        .ok_or("step-4-participation:observed=no-target-progress;need=positive-kill-progress")?;
    if varint_field(&progress, 2)?.is_none_or(|count| count == 0) {
        return Err(
            "step-4-participation:observed=zero-kill-count;need=positive-kill-progress".into(),
        );
    }
    let chest = command(&mut client, wire::COMMAND_TYPE_BOSS_REWARD_OPEN, &[]).await?;
    let settled_reward = nested_entry(&chest.payload, 2, BOSS)?;
    let reward_count = settled_reward
        .as_ref()
        .map(|entry| varint_field(entry, 5))
        .transpose()?
        .flatten();
    if chest.status != CommandStatus::Accepted || reward_count.is_none_or(|count| count == 0) {
        return Err(format!(
            "step-4-reward:observed={chest:?};need=personal-participation-reward-group"
        )
        .into());
    }
    let settled_reward = settled_reward.ok_or(
        "step-4-reward:observed=missing-target-group;need=personal-participation-reward-group",
    )?;
    evidence("W02_CHECK step=4 status=PASS observed=cooldown-and-personal-reward");

    // The defeated handoff returns at the authored exit. The repeat probe must stand
    // on a lever square so the cooldown is the reason for refusal.
    walk_to_boss_lever(&mut client, clients).await?;
    let before_repeat_session = client.join_snapshot().game_session_id;
    let before_repeat_position = client.world_spatial().actor_position;
    let payload =
        wire::encode_boss_key(BOSS).map_err(|error| format!("step-5-lever-encode:{error:?}"))?;
    let repeated = command(&mut client, wire::COMMAND_TYPE_BOSS_LEVER, &payload).await?;
    let repeated_reply = wire::decode_boss_activity_reply(&repeated.payload)
        .map_err(|error| format!("step-5-cooldown-refusal-decode:{error:?}"))?;
    if repeated.status != CommandStatus::Rejected
        || repeated_reply != wire::BossActivityReply::Refused(wire::BossActivityRefusal::Cooldown)
    {
        return Err(format!("step-5-cooldown-refusal:observed-status={:?},reply={repeated_reply:?};need=typed-Cooldown", repeated.status).into());
    }
    if client.join_snapshot().game_session_id != before_repeat_session
        || client.world_spatial().actor_position != before_repeat_position
    {
        return Err(format!(
            "step-5-no-second-handoff:observed-reply={repeated_reply:?},session-changed={},position-changed={};need=Cooldown-without-Instance-handoff",
            client.join_snapshot().game_session_id != before_repeat_session,
            client.world_spatial().actor_position != before_repeat_position
        )
        .into());
    }
    evidence(
        "W02_CHECK step=5 status=PASS observed=typed-cooldown-refusal-without-session-or-position-handoff",
    );

    let old_session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.status != CommandStatus::Accepted {
        return Err(format!("step-6-logout:observed={logout:?};need=accepted-logout").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xb3).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("step-6-fresh-admission:observed=reused-session;need=new-session".into());
    }
    let durable = cooldowns(&mut fresh).await?;
    let durable_chest = command(&mut fresh, wire::COMMAND_TYPE_BOSS_REWARD_OPEN, &[]).await?;
    let durable_cooldown = nested_entry(&durable, 2, BOSS)?;
    let durable_reward = nested_entry(&durable_chest.payload, 2, BOSS)?;
    if durable_cooldown.as_ref() != Some(&settled_cooldown)
        || durable_chest.status != CommandStatus::Accepted
        || durable_reward.as_ref() != Some(&settled_reward)
    {
        return Err(format!("step-6-durable-recheck:observed-cooldown-match={},reward-match={},chest-status={:?};need=same-deadline-and-personal-reward-group", durable_cooldown.as_ref() == Some(&settled_cooldown), durable_reward.as_ref() == Some(&settled_reward), durable_chest.status).into());
    }
    evidence("W02_CHECK step=6 status=PASS observed=durable-state-after-fresh-admission");
    Ok(())
}

fn contains_key(payload: &[u8], key: &str) -> bool {
    payload
        .windows(key.len())
        .any(|window| window == key.as_bytes())
}

fn read_varint(payload: &[u8], at: &mut usize) -> TestResult<u64> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let byte = *payload
            .get(*at)
            .ok_or("boss-reply:observed=truncated-varint")?;
        *at += 1;
        if shift == 63 && byte > 1 {
            return Err("boss-reply:observed=overflowing-varint".into());
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("boss-reply:observed=overlong-varint".into())
}

fn nested_entry(payload: &[u8], field: u64, key: &str) -> TestResult<Option<Vec<u8>>> {
    let mut at = 0;
    while at < payload.len() {
        let tag = read_varint(payload, &mut at)?;
        match tag & 7 {
            0 => {
                read_varint(payload, &mut at)?;
            }
            2 => {
                let len = usize::try_from(read_varint(payload, &mut at)?)?;
                let end = at
                    .checked_add(len)
                    .filter(|end| *end <= payload.len())
                    .ok_or("boss-reply:observed=truncated-field")?;
                if tag >> 3 == field && contains_key(&payload[at..end], key) {
                    return Ok(Some(payload[at..end].to_vec()));
                }
                at = end;
            }
            wire_type => {
                return Err(
                    format!("boss-reply:observed=unsupported-wire-type-{wire_type}").into(),
                );
            }
        }
    }
    Ok(None)
}

fn varint_field(payload: &[u8], field: u64) -> TestResult<Option<u64>> {
    let mut at = 0;
    while at < payload.len() {
        let tag = read_varint(payload, &mut at)?;
        match tag & 7 {
            0 => {
                let value = read_varint(payload, &mut at)?;
                if tag >> 3 == field {
                    return Ok(Some(value));
                }
            }
            2 => {
                let len = usize::try_from(read_varint(payload, &mut at)?)?;
                at = at
                    .checked_add(len)
                    .filter(|end| *end <= payload.len())
                    .ok_or("boss-reply:observed=truncated-field")?;
            }
            wire_type => {
                return Err(
                    format!("boss-reply:observed=unsupported-wire-type-{wire_type}").into(),
                );
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(
            evidence_token("step 2: Refused(Position)"),
            "step-2:-Refused-Position-"
        );
    }

    #[test]
    fn cooldown_key_detection_requires_contiguous_wire_text() {
        assert!(contains_key(b"\x0a\x1boteryn:boss.faceless_bane", BOSS));
        assert!(!contains_key(b"oteryn:boss.other", BOSS));
    }
}

#[cfg(test)]
mod delta_review_tests {
    use super::*;
    #[test]
    fn defeated_return_uses_authored_exit_and_wait() {
        let exit = ActorPosition {
            x: 33618,
            y: 32523,
            floor: 15,
        };
        let (budget, goal) = return_expectation(exit, Duration::from_secs(60));
        assert_eq!(goal, exit);
        assert!(budget > Duration::from_secs(60));
    }
}

#[cfg(test)]
#[test]
fn look_refusal_does_not_establish_actor_absence() {
    use oteryn_protocol_oteryn::item_look::{ItemLookRefusal, ItemLookResult};
    for refusal in [ItemLookRefusal::RateLimited, ItemLookRefusal::NotAvailable] {
        assert!(classify_actor_name(ItemLookResult::Refused(refusal), "Rashid").is_err());
    }
}
