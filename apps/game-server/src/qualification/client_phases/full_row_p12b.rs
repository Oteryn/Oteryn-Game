//! P12b exercises the native account badge award through the Cyclopedia request.
//! The dedicated account starts fresh. Qualification supplies the level input,
//! and only the wire request may award the badge. Original award rows must
//! survive terminal logout and fresh admission before another refresh.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::cyclopedia_character::{
    Badge, CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_BADGES,
};
use sqlx::Connection;

// Native Cyclopedia contract: a total level over 500 earns Global Player grade 1.
// Keep the expected wire ID independent of the production eligibility evaluator.
pub(super) const ROW_INDEX: usize = 278;
const EXPECTED_BADGES: [u8; 1] = [9];

/// The reply is `count:u8` followed by `count` strictly ascending known badge ids.
fn decode_badges(payload: &[u8]) -> Result<Vec<u8>, String> {
    let (count, ids) = payload.split_first().ok_or("empty-reply")?;
    if usize::from(*count) != ids.len() {
        return Err(format!("count:{count},bytes:{}", ids.len()));
    }
    if ids.iter().any(|id| Badge::from_id(*id).is_none()) {
        return Err("unknown-badge-id".to_owned());
    }
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("ids-not-ascending".to_owned());
    }
    Ok(ids.to_vec())
}

async fn reply_badges(client: &mut DevClientSession) -> TestResult<Vec<u8>> {
    expect(
        client
            .selected_capabilities()
            .contains(&CAPABILITY_CYCLOPEDIA_CHARACTER_V1),
        "capability",
        "needed=capability-199-selected-at-join",
    )?;
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            COMMAND_TYPE_CYCLOPEDIA_BADGES,
            &[],
        )
        .await?;
    require_mechanic_acceptance(reply.status, "badges")?;
    Ok(decode_badges(&reply.payload).map_err(|error| format!("step=decode;observed={error}"))?)
}

async fn stored_badges(clients: &SeamClients<'_>) -> TestResult<Vec<(i16, i64)>> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_as(
        "SELECT badge, earned_at_unix_ms FROM game_account_badges \
         WHERE account_id=(SELECT account_id FROM game_character_roots \
         WHERE character_id=encode($1,'hex')::uuid) ORDER BY badge",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_all(&mut connection)
    .await?)
}

async fn seed_level(
    clients: &SeamClients<'_>,
    client: &DevClientSession,
    profile: &str,
    expected: (i64, i64),
) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let vocation: String = sqlx::query_scalar(
        "SELECT vocation FROM game_character_build_state
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut database)
    .await?;
    expect(vocation == "knight", "bootstrap-vocation", &vocation)?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        profile.into(),
    )
    .await?;
    let progression: (i64, i64) = sqlx::query_as(
        "SELECT level, total_experience FROM game_character_progression_state
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut database)
    .await?;
    expect(progression == expected, "seeded-progression", (expected, progression))
}

async fn expect_account_levels(clients: &SeamClients<'_>, expected: &[i64]) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let levels: Vec<Option<i64>> = sqlx::query_scalar(
        "SELECT p.level
         FROM game_character_roots r
         LEFT JOIN game_character_progression_state p USING (character_id)
         WHERE r.account_id=$1::uuid ORDER BY 1 LIMIT 3",
    )
    .bind(&clients.accounts[0])
    .fetch_all(&mut database)
    .await?;
    expect(
        levels.iter().copied().eq(expected.iter().copied().map(Some)),
        "account-levels",
        (expected, &levels),
    )
}

// A real PvE death changes only the eligibility input. The badge is read on
// either side through the same player route and compared to its original row.
async fn lose_eligibility(clients: &SeamClients<'_>, client: &mut DevClientSession) -> TestResult {
    let world = full_e2ef7::qualification_world(clients)?;
    let town = tile(client.world_spatial().actor_position)?;
    let path = route(world, town, |at| {
        at.floor == town.floor
            && world.base().tile(at.x, at.y, at.floor)
                .is_some_and(|cell| cell.walkable() && !cell.protection_zone())
    })
    .ok_or("P12b death setup has no reachable combat tile")?;
    walk(client, world, &path).await?;
    let encounter = tile(client.world_spatial().actor_position)?;
    expect(encounter != town, "death-combat-tile", encounter)?;
    client.take_events();
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let dragon = parse_spawned_entity(&request_spawn(session, "oteryn:creature.dragon".into()).await?)?;
    evidence(&format!("P12b eligibility-loss fixture=dragon attacker={dragon:?} position={encounter:?}"));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut damaged_by_dragon = false;
    loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::AnalyserAttributedFacts(delta) = event {
                for fact in delta.value {
                    if let Some(attacker) = fact.attacker {
                        expect(attacker == dragon, "death-attacker", attacker)?;
                        damaged_by_dragon = true;
                    }
                }
            }
        }
        if damaged_by_dragon && tile(client.world_spatial().actor_position)? == town {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("P12b eligibility loss did not respawn: dragon_damage={damaged_by_dragon}").into());
        }
    }

    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let receipt: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT level_before, level_after, experience_before, experience_after
         FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid
         ORDER BY committed_at DESC, death_occurrence_id DESC LIMIT 1",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut database)
    .await?;
    expect(receipt == (2, 1, 100, 0), "eligibility-loss-receipt", receipt)?;
    expect_account_levels(clients, &[1, 499]).await?;
    let combat_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while !client.session_mut().combat_state().is_some_and(|state| !state.in_fight) {
        if tokio::time::Instant::now() >= combat_deadline {
            return Err("P12b combat lock did not clear after leaving the dragon".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    Ok(())
}

async fn journey(scope: &SeamClients<'_>) -> TestResult<Verdict> {
    let fixture = character_prep::dedicated_named_character(
        scope, "NODE_BOOT_P12B_ACCOUNT", Some("Badge Knight"),
    ).await?;
    let peer = character_prep::dedicated_named_character(
        scope, "NODE_BOOT_P12B_ACCOUNT", Some("Badge Peer"),
    ).await?;
    let peer_clients = &peer.clients(scope);
    let clients = &fixture.clients(scope);
    let account = crate::node::uuid_bytes(&clients.accounts[0]).ok_or("invalid q_p12b account")?;
    let mut timestamp = [0; 8];
    timestamp[2..].copy_from_slice(&account[..6]);
    let created_ms = u64::from_be_bytes(timestamp);
    let now_ms = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    expect(
        now_ms
            .checked_sub(created_ms)
            .is_some_and(|age| age < 86_400_000),
        "fresh-account",
        (created_ms, now_ms),
    )?;

    let before = stored_badges(clients).await?;
    expect(before.is_empty(), "badge-pre-admission", &before)?;
    let mut peer_at_birth = connect_character(peer_clients, 0xc0, 0).await?;
    logout(peer_clients, &mut peer_at_birth).await?;
    let mut client = connect_character(clients, 0xc1, 0).await?;
    let unqualified = reply_badges(&mut client).await?;
    expect(
        unqualified.is_empty(),
        "unqualified-badge-reply",
        &unqualified,
    )?;
    let unqualified_stored = stored_badges(clients).await?;
    expect(
        unqualified_stored.is_empty(),
        "unqualified-badge-storage",
        &unqualified_stored,
    )?;

    character_prep::declare_seeded(
        "P12b.full", "knight-level-499,knight-peer-level-2", "Cyclopedia level-sum badge",
    );
    seed_level(clients, &client, "p12b-knight", (499, 2_046_099_400)).await?;
    expect_account_levels(clients, &[1, 499]).await?;
    let boundary = reply_badges(&mut client).await?;
    expect(boundary.is_empty(), "level-sum-500-reply", &boundary)?;
    let boundary_rows = stored_badges(clients).await?;
    expect(boundary_rows.is_empty(), "level-sum-500-storage", &boundary_rows)?;
    logout(clients, &mut client).await?;

    let mut client = connect_character(peer_clients, 0xc3, 0).await?;
    let session = client.join_snapshot().game_session_id;
    seed_level(peer_clients, &client, "p12b-peer", (2, 100)).await?;
    expect_account_levels(peer_clients, &[2, 499]).await?;
    let clients = peer_clients;
    let before_award = stored_badges(clients).await?;
    expect(before_award.is_empty(), "badge-pre-request", &before_award)?;

    let reply = reply_badges(&mut client).await?;
    expect(
        reply == EXPECTED_BADGES,
        "qualifying-badges-granted",
        &reply,
    )?;
    let awarded = stored_badges(clients).await?;
    expect(
        awarded
            .iter()
            .map(|&(id, _)| id)
            .eq(EXPECTED_BADGES.map(i16::from)),
        "reply-vs-durable",
        &awarded,
    )?;
    logout(clients, &mut client).await?;
    let offline = stored_badges(clients).await?;
    expect(
        offline == awarded,
        "durable-before-relog",
        (&awarded, &offline),
    )?;

    let mut fresh = connect_character(clients, 0xc2, 0).await?;
    expect(
        fresh.join_snapshot().game_session_id != session,
        "fresh-session",
        fresh.join_snapshot().game_session_id,
    )?;
    let admitted = stored_badges(clients).await?;
    expect(
        admitted == awarded,
        "durable-after-admission",
        (&awarded, &admitted),
    )?;
    let again = reply_badges(&mut fresh).await?;
    expect(again == EXPECTED_BADGES, "reply-after-relog", &again)?;
    let reread = stored_badges(clients).await?;
    expect(
        reread == awarded,
        "durable-after-relog",
        (&awarded, &reread),
    )?;
    lose_eligibility(clients, &mut fresh).await?;
    let below_threshold = reply_badges(&mut fresh).await?;
    expect(below_threshold == EXPECTED_BADGES, "permanent-badge-after-level-loss", &below_threshold)?;
    let retained = stored_badges(clients).await?;
    expect(retained == awarded, "original-badge-after-level-loss", (&awarded, &retained))?;
    logout(clients, &mut fresh).await?;
    let released = stored_badges(clients).await?;
    expect(
        released == awarded,
        "durable-after-final-logout",
        (&awarded, &released),
    )?;
    Ok((
        "PASS",
        format!("badges={reply:?};account-levels=499+2;level-sum-500-empty;pre-state-empty;original-awards-survive-relog-and-level-loss"),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P12b.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

/// One evidence token: no whitespace, bounded length.
fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=<>,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .take(240)
        .collect()
}

fn expect(ok: bool, step: &str, observed: impl std::fmt::Debug) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(format!("step={step};observed={observed:?}").into())
    }
}

async fn logout(clients: &SeamClients<'_>, client: &mut DevClientSession) -> TestResult {
    let session = client.join_snapshot().game_session_id;
    let (result, detail, _, _) = logout_once(client).await?;
    expect(result == Some(LogoutResult::Accepted), "logout", detail)?;

    // Accepted is sent after the release transaction. Inspect that exact session
    // immediately, while the client is still in scope. A loss/grace release
    // retains its control-loss epoch and cannot satisfy this check.
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let release: (i16, bool, bool) = sqlx::query_as(
        "SELECT session_state, current_transport_ref IS NULL, control_loss_epoch IS NULL
         FROM game_durability_reconnect_sessions
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_one(&mut database)
    .await?;
    expect(release == (3, true, true), "explicit-terminal-logout", release)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }

    #[test]
    fn badge_reply_decoder_checks_count_order_and_ids() {
        assert_eq!(decode_badges(&[0]), Ok(vec![]));
        assert!(decode_badges(&[]).is_err());
        assert!(decode_badges(&[2, 1]).is_err());
        assert!(decode_badges(&[2, 2, 1]).is_err());
        assert!(decode_badges(&[1, 250]).is_err());
        let first = Badge::ALL[0].id();
        assert_eq!(decode_badges(&[1, first]), Ok(vec![first]));
    }
}
