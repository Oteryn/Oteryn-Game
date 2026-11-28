//! G34: ENGNPC NPC/quest receipts (#3050). A quest NPC offer commits its track write and the
//! quest receipt atomically (`crates/gameplay/dispatch/src/gameplay_transport/npc_command/
//! reward_callbacks.rs`).
//! Start Hanna's citizen-hat mission through the authored prompt and confirmation.
//! Require the NPC conversation reply, then verify each track and its bound receipt
//! before logout and after fresh admission.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{CommandStatus, npc_service as wire};
use sqlx::Connection;

const NPC: &str = "oteryn:npc.hanna";
// Crystal Hanna walks within two tiles of her placement.
const HANNA_WANDER_RADIUS: u32 = 2;
const TRANSITION: &str = "oteryn:quest-reward/qb-citizen-hat-start";

const TRACKS: [&str; 2] = [
    "oteryn:quest-progress/quest/u7_8/citizen_outfits/addon_hat",
    "oteryn:quest-progress/quest/u7_8/citizen_outfits/mission_hat",
];
const PROMPT: &str = "Pretty, isn't it? My friend Amber taught me how to make it, but I could help you with one if you like. What do you say?";
const CONFIRMATION: &str =
    "Bring me a legion helmet, 100 chicken feathers, and 50 honeycombs so I can make your hat.";
const RETURN_PROMPT: &str = "Oh, you're back already? Did you bring a legion helmet, 100 chicken feathers and 50 honeycombs?";
const HAT_GRANT: &str = "oteryn:outfit.citizen/quest-grant/2";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G34.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn dialogue_matches(lines: &[String], expected: &str) -> bool {
    lines == [expected]
}

fn greeting(client: &DevClientSession) -> TestResult<String> {
    let entities = client.world_entities().ok_or("missing own entity view")?;
    let name = entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .and_then(|entity| entity.display_name)
        .ok_or("missing own display name")?;
    Ok(format!(
        "Oh, please come in, {}. What do you need? Have a look at my wonderful {{offers}} in gems and jewellery.",
        name.as_str().map_err(|_| "invalid own display name")?
    ))
}

async fn require_hat_ungranted(clients: &SeamClients<'_>) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let granted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u \
         JOIN game_character_roots r ON r.account_id = u.account_id \
         WHERE r.character_id = encode($1, 'hex')::uuid AND u.unlock_key = $2)",
    )
    .bind(clients.characters[1].as_slice())
    .bind(HAT_GRANT)
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    if granted {
        return Err("step=appearance;observed=citizen-hat-granted-before-material-hand-in".into());
    }
    Ok(())
}

async fn track_value(clients: &SeamClients<'_>, track: &str) -> TestResult<Option<i64>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let value: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE character_id = encode($1, 'hex')::uuid AND track_key = $2",
    )
    .bind(clients.characters[1].as_slice())
    .bind(track)
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    Ok(value)
}

fn receipt_effect_matches(keys: &[String], before: &[i64], after: &[i64], completes: bool) -> bool {
    !completes
        && keys.len() == TRACKS.len()
        && TRACKS
            .iter()
            .all(|track| keys.iter().any(|key| key == track))
        && before == [0, 0]
        && after == [1, 1]
}

/// Identify the single command receipt bound to the track’s exact before and after values.
async fn bound_receipt(
    clients: &SeamClients<'_>,
    track: &str,
    before: i64,
    session: &[u8; 16],
    command: u64,
) -> TestResult<Option<String>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    type ReceiptRow = (String, Vec<String>, Vec<i64>, Vec<i64>, bool);
    let receipts: Vec<ReceiptRow> = sqlx::query_as(
        "SELECT concat_ws(':', r.cause_id, r.cause_ordinal, r.committed_character_revision), \
                r.track_keys, r.values_before, r.values_after, r.completes \
         FROM game_character_quest_receipts r \
         JOIN game_character_quest_tracks t ON t.character_id = r.character_id AND t.track_key = $2 \
         WHERE r.character_id = encode($1, 'hex')::uuid \
           AND r.committed_character_revision = t.committed_character_revision \
           AND t.quest_key = r.quest_key AND t.value = $3 \
           AND r.values_after[array_position(r.track_keys, $2)] = $3 \
           AND r.values_before[array_position(r.track_keys, $2)] = $4 \
           AND r.transition_key = $5 AND r.cause_kind = 'command' \
           AND r.cause_id = encode($6, 'hex')::uuid \
           AND r.cause_ordinal = $7::text::numeric LIMIT 2",
    )
    .bind(clients.characters[1].as_slice())
    .bind(track)
    .bind(1_i64)
    .bind(before)
    .bind(TRANSITION)
    .bind(session.as_slice())
    .bind(command.to_string())
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    match receipts.as_slice() {
        [] => Ok(None),
        [(identity, keys, before, after, completes)] => {
            if !receipt_effect_matches(keys, before, after, *completes) {
                return Err(format!(
                    "step=receipt;observed=unexpected-effects;keys={keys:?};before={before:?};after={after:?};completes={completes}"
                ).into());
            }
            Ok(Some(identity.clone()))
        }
        _ => Err(format!(
            "track={track};expected=one-bound-receipt;observed={}",
            receipts.len()
        )
        .into()),
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(())
}

fn conversation_batch_matches(
    replies: &[wire::NpcConversation],
    actor: oteryn_dev_client::EntityRef,
    expected_line: &str,
) -> bool {
    matches!(replies, [reply]
        if reply.open == Some((actor, wire::NpcConversationState::Open))
            && dialogue_matches(&reply.lines, expected_line))
}

async fn talk(
    client: &mut DevClientSession,
    actor: oteryn_dev_client::EntityRef,
    text: &str,
    expected_line: &str,
) -> TestResult<u64> {
    client.take_events();
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("step=talk;encode={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || wire::decode_npc_intent_result(&reply.payload).ok()
            != Some(wire::NpcIntentDisposition::Ok)
    {
        return Err(format!("step=talk;text={text};observed={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let replies: Vec<_> = client
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                SessionEvent::NpcConversation(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    Some(delta.value)
                }
                _ => None,
            })
            .collect();
        if !replies.is_empty() {
            if !conversation_batch_matches(&replies, actor, expected_line) {
                return Err(format!(
                    "step=reply;npc={NPC};keyword={text};observed={replies:?};expected={expected_line:?}"
                )
                .into());
            }
            return Ok(reply.command_id);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=reply;npc={NPC};keyword={text};observed=expected-conversation-effect-missing"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == NPC)
        .ok_or("step=select;needed=hanna-placement")?;
    let mut client = connect_world(clients, 0xe4).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        npc.placements.iter().any(|p| {
            p.floor == at.floor
                && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y))) <= wire::TALK_RANGE_TILES
        })
    })
    .ok_or("step=select;needed=walkable-hanna-approach")?;
    if path.is_empty() {
        return Err("step=select;observed=already-at-approach".into());
    }
    let mut before = Vec::new();
    for track in TRACKS {
        let value = track_value(clients, track).await?;
        if value.unwrap_or(0) != 0 {
            let _ = logout(&mut client).await;
            return Err(format!("step=initial;track={track};observed=already:{value:?}").into());
        }
        before.push(value);
    }
    require_hat_ungranted(clients).await?;
    let mut wait_ms = 0;
    for direction in path {
        if wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(wait_ms)))
                .await?;
        }
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction)
            .ok_or("step=walk;observed=missing-planned-landing")?;
        let outcome = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.status != CommandStatus::Accepted
            || outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.as_ref().is_none_or(|delta| {
                delta.server_sequence <= outcome.result_server_sequence
                    || tile(delta.value.actor_position).ok() != Some(expected)
            })
            || actual != expected
            || actual == from
        {
            return Err(format!(
                "step=walk;from={from:?};direction={direction:?};expected={expected:?};actual={actual:?};disposition={:?}",
                outcome.disposition
            )
            .into());
        }
        wait_ms = outcome.step_wait_ms;
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    let hanna = |client: &DevClientSession| -> TestResult<oteryn_dev_client::EntityRef> {
        let player = client.world_spatial().actor_position;
        let mut visible_hanna = client
            .world_entities()
            .ok_or("step=walk;observed=missing-entity-view")?
            .iter()
            .filter(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && entity
                        .display_name
                        .is_some_and(|name| name.as_str() == Ok("Hanna"))
                    && entity.position.floor == player.floor
                    && entity
                        .position
                        .x
                        .abs_diff(player.x)
                        .max(entity.position.y.abs_diff(player.y))
                        <= wire::TALK_RANGE_TILES
                    && npc.placements.iter().any(|p| {
                        entity.position.floor == -i16::from(p.floor)
                            && entity
                                .position
                                .x
                                .abs_diff(i32::from(p.x))
                                .max(entity.position.y.abs_diff(i32::from(p.y)))
                                <= HANNA_WANDER_RADIUS
                    })
            });
        let actor = visible_hanna
            .next()
            .ok_or("step=walk;observed=npc-not-visible")?
            .entity;
        if visible_hanna.next().is_some() {
            return Err("step=walk;observed=ambiguous-hanna".into());
        }
        Ok(actor)
    };
    let actor = hanna(&client)?;
    let initial_session = *client.join_snapshot().game_session_id.as_bytes();
    let expected_greeting = greeting(&client)?;
    talk(&mut client, actor, "hi", &expected_greeting).await?;
    let prompt_command = talk(&mut client, actor, "addon", PROMPT).await?;
    for (track, prior) in TRACKS.into_iter().zip(&before) {
        if track_value(clients, track).await? != *prior {
            return Err(
                format!("step=pre-confirmation;track={track};observed=premature-write").into(),
            );
        }
    }
    let confirmation = talk(&mut client, actor, "yes", CONFIRMATION).await?;
    require_hat_ungranted(clients).await?;
    let mut committed_receipts = Vec::new();
    for (track, prior) in TRACKS.into_iter().zip(&before) {
        let value = track_value(clients, track).await?;
        if value != Some(1) {
            return Err(format!("step=receipt;track={track};observed={value:?};expected=1").into());
        }
        committed_receipts.push(
            bound_receipt(
                clients,
                track,
                prior.unwrap_or(0),
                &initial_session,
                confirmation,
            )
            .await?
            .ok_or("step=receipt;observed=missing-confirmation-receipt")?,
        );
    }
    if committed_receipts.windows(2).any(|pair| pair[0] != pair[1]) {
        return Err("step=receipt;observed=tracks-committed-by-different-commands".into());
    }
    // A real earlier command from the same admission must not match the confirmation receipt.
    let wrong_command = bound_receipt(
        clients,
        TRACKS[0],
        before[0].unwrap_or(0),
        &initial_session,
        prompt_command,
    )
    .await?;
    if wrong_command.is_some() {
        return Err("step=receipt;wrong-command-negative-check-failed".into());
    }
    let approach = client.world_spatial().actor_position;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xe5).await?;
    if fresh.join_snapshot().game_session_id.as_bytes() == &initial_session
        || fresh.world_spatial().actor_position != approach
    {
        return Err("step=fresh;expected=new-session-at-saved-approach".into());
    }
    if bound_receipt(
        clients,
        TRACKS[0],
        before[0].unwrap_or(0),
        fresh.join_snapshot().game_session_id.as_bytes(),
        confirmation,
    )
    .await?
    .is_some()
    {
        return Err("step=receipt;wrong-session-negative-check-failed".into());
    }
    require_hat_ungranted(clients).await?;
    let actor = hanna(&fresh)?;
    let expected_greeting = greeting(&fresh)?;
    talk(&mut fresh, actor, "hi", &expected_greeting).await?;
    talk(&mut fresh, actor, "hat", RETURN_PROMPT).await?;
    require_hat_ungranted(clients).await?;
    for ((track, prior), committed) in TRACKS.into_iter().zip(before).zip(committed_receipts) {
        let value = track_value(clients, track).await?;
        let receipt = bound_receipt(
            clients,
            track,
            prior.unwrap_or(0),
            &initial_session,
            confirmation,
        )
        .await?;
        if value != Some(1) || receipt.as_ref() != Some(&committed) {
            return Err(format!(
                "step=fresh;track={track};observed={value:?};expected=1;same-commit={}",
                receipt.as_ref() == Some(&committed)
            )
            .into());
        }
    }
    logout(&mut fresh).await?;
    Ok(format!(
        "npc={NPC};transition={TRANSITION};durable-tracks={}",
        TRACKS.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialogue_rejects_empty_extra_wrong_quantities_and_unrestored_progress() {
        for (lines, expected) in [
            (vec![], "greeting"),
            (vec!["wrong greeting"], "greeting"),
            (vec![PROMPT, "extra"], PROMPT),
            (
                vec![
                    "Bring me a legion helmet, 1100 chicken feathers, and 150 honeycombs so I can make your hat.",
                ],
                CONFIRMATION,
            ),
            (vec![CONFIRMATION, "extra"], CONFIRMATION),
            (vec![PROMPT], RETURN_PROMPT),
        ] {
            assert!(!dialogue_matches(
                &lines.into_iter().map(str::to_owned).collect::<Vec<_>>(),
                expected
            ));
        }
        assert!(dialogue_matches(&[RETURN_PROMPT.into()], RETURN_PROMPT));
    }
    #[test]
    fn receipt_rejects_extra_tracks_and_premature_completion() {
        let mut keys = TRACKS.map(str::to_owned).to_vec();
        assert!(receipt_effect_matches(&keys, &[0, 0], &[1, 1], false));
        assert!(!receipt_effect_matches(&keys, &[0, 0], &[1, 1], true));
        keys.push("unexpected-track".into());
        assert!(!receipt_effect_matches(
            &keys,
            &[0, 0, 0],
            &[1, 1, 1],
            false
        ));
    }

    #[test]
    fn conversation_batch_rejects_conflicting_followup() {
        let actor = oteryn_dev_client::EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let correct = wire::NpcConversation {
            open: Some((actor, wire::NpcConversationState::Open)),
            lines: vec![RETURN_PROMPT.into()],
        };
        assert!(conversation_batch_matches(
            std::slice::from_ref(&correct),
            actor,
            RETURN_PROMPT
        ));
        let conflict = wire::NpcConversation {
            lines: vec![PROMPT.into()],
            ..correct.clone()
        };
        assert!(!conversation_batch_matches(
            &[correct, conflict],
            actor,
            RETURN_PROMPT
        ));
    }
}
