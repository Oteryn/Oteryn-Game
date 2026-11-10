//! P12b (Cyclopedia badges, Crystal `PlayerBadge::checkAndUpdateNewBadges`):
//! 1. Admit the node-boot character, require capability 199 and send the empty badges request.
//! 2. Decode the accepted reply (count, ascending known badge ids).
//! 3. Compare it with the durable truth read through read-only SQL: the earned rows of the
//!    account (`game_account_badges`) must equal the reply and must include every badge the
//!    account facts (age, loyalty, level sum, vocation levels) qualify for.
//! 4. Log out, freshly admit and require the same badges again.
//!
//! An account that meets no global badge rule has nothing to show, so the row then reports the
//! missing precondition instead of passing an empty list.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::{
    Badge, BadgeFacts, BaseVocation, CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
    COMMAND_TYPE_CYCLOPEDIA_BADGES, newly_earned,
};
use sqlx::{Connection, Row};

const FACTS_SQL: &str = "WITH me AS (SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid), \
 chars AS (SELECT p.level, CASE b.vocation WHEN 'elder_druid' THEN 'druid' WHEN 'elite_knight' THEN 'knight' WHEN 'exalted_monk' THEN 'monk' \
   WHEN 'royal_paladin' THEN 'paladin' WHEN 'master_sorcerer' THEN 'sorcerer' ELSE b.vocation END AS base \
   FROM game_character_roots r JOIN me ON me.account_id=r.account_id \
   JOIN game_character_progression_state p ON p.character_id=r.character_id \
   LEFT JOIN game_character_build_state b ON b.character_id=r.character_id) \
 SELECT \
 COALESCE((SELECT extract(year FROM age(to_timestamp($2/1000.0), to_timestamp((('x'||lpad(substr(replace(me.account_id::text,'-',''),1,12),16,'0'))::bit(64)::bigint)/1000.0)))::bigint FROM me),0) AS account_years, \
 COALESCE((SELECT l.loyalty_points FROM game_account_loyalty_projections l JOIN me ON me.account_id=l.account_id),0)::bigint AS loyalty_points, \
 COALESCE((SELECT sum(level) FROM chars),0)::bigint AS level_sum, \
 ARRAY(SELECT COALESCE((SELECT max(level) FROM chars WHERE base=v.key),0)::bigint FROM unnest($3::text[]) WITH ORDINALITY v(key,n) ORDER BY v.n) AS vocation_levels";

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
    expect(
        reply.status == CommandStatus::Accepted,
        "badges",
        reply.status,
    )?;
    Ok(decode_badges(&reply.payload).map_err(|error| format!("step=decode;observed={error}"))?)
}

async fn stored_badges(clients: &SeamClients<'_>) -> TestResult<Vec<u8>> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let rows: Vec<i16> = sqlx::query_scalar(
        "SELECT badge FROM game_account_badges WHERE account_id=(SELECT account_id FROM game_character_roots \
         WHERE character_id=encode($1,'hex')::uuid) ORDER BY badge",
    )
    .bind(clients.characters[1].as_slice())
    .fetch_all(&mut connection)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|id| u8::try_from(id).ok())
        .collect())
}

async fn expected_badges(clients: &SeamClients<'_>) -> TestResult<Vec<u8>> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let now_ms = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let row = sqlx::query(FACTS_SQL)
        .bind(clients.characters[1].as_slice())
        .bind(now_ms)
        .bind(BaseVocation::KEYS.as_slice())
        .fetch_one(&mut connection)
        .await?;
    let count = |value: i64| u32::try_from(value.max(0)).unwrap_or(u32::MAX);
    let levels: Vec<i64> = row.try_get("vocation_levels")?;
    let mut vocation_levels = [0; 5];
    for (slot, value) in vocation_levels.iter_mut().zip(levels) {
        *slot = count(value);
    }
    let facts = BadgeFacts {
        account_years: count(row.try_get("account_years")?),
        loyalty_points: row.try_get("loyalty_points")?,
        level_sum: row.try_get("level_sum")?,
        vocation_levels,
    };
    Ok(newly_earned(facts, &[])
        .into_iter()
        .map(Badge::id)
        .collect())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let expected = expected_badges(clients).await?;
    let mut client = connect_world(clients, 0xc1).await?;
    let reply = reply_badges(&mut client).await?;
    let stored = stored_badges(clients).await?;
    expect(reply == stored, "reply-vs-durable", (&reply, &stored))?;
    expect(
        expected.iter().all(|id| reply.contains(id)),
        "qualifying-badges-granted",
        (&expected, &reply),
    )?;
    logout(&mut client).await?;

    let mut fresh = connect_world(clients, 0xc2).await?;
    let again = reply_badges(&mut fresh).await?;
    expect(again == reply, "reply-after-relog", (&reply, &again))?;
    expect(
        stored_badges(clients).await? == reply,
        "durable-after-relog",
        "rows-changed",
    )?;
    logout(&mut fresh).await?;
    if reply.is_empty() {
        return Ok((
            "FAIL",
            format!(
                "step=badges;observed=empty-accepted-list;needed=account-meeting-a-global-badge-rule;facts-qualify:{}",
                expected.len()
            ),
        ));
    }
    Ok((
        "PASS",
        format!("badges={reply:?};durable-and-relog-validated"),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(120), journey(clients)).await;
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

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    expect(
        reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        "logout",
        reply.result,
    )
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
