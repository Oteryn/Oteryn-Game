//! G17 returner reward query and claim (capability 182, commands 6124 and 6125).
//! 1. Admit and log out a dedicated character, then age its terminal history in the disposable database.
//! 2. With no entitlement (state 0) a claim for an unknown identity must be refused and leave
//!    the inventory and the state unchanged; the row then fails naming the precondition,
//!    because an entitlement needs twelve months of inactivity evidence no player route makes.
//! 3. With an entitlement (state 1) claim its identity: the reply must be disposition 1 carrying
//!    that identity, the same session then reads state 2, a second claim is disposition 2, and
//!    the durable claim receipt joined to its minted item instance must exist (read-only SQL).
//!    The reward is delivered to the reward inbox, not the backpack, so the inventory is not the
//!    evidence. After logout and a fresh admission the receipt must still exist and the
//!    entitlement must not be offered again (state 2 is tied to the source session).

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_reward::{self as daily, returner};
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "returner-reward-claimed-receipt-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G17.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={detail}").into());
    }
    Ok(())
}

const CAPABILITY: u32 = daily::CAPABILITY_DAILY_REWARD_V2;
const BOGUS: [u8; 16] = [
    0x01, 0x93, 0x5f, 0x3b, 0x2c, 0x4d, 0x7a, 0x11, 0x8b, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
];

async fn state(client: &mut DevClientSession) -> TestResult<returner::Reply> {
    let payload = accepted(client, CAPABILITY, returner::QUERY, &[]).await?;
    Ok(returner::Reply::decode(&payload)?)
}

/// Claim reply: field 1 = disposition, field 2 (when claimed) = the 16-byte transaction id.
fn claim_reply_matches(payload: &[u8], disposition: u8, id: Option<[u8; 16]>) -> bool {
    let mut expected = vec![0x08, disposition];
    if let Some(id) = id {
        expected.extend_from_slice(&[0x12, 16]);
        expected.extend_from_slice(&id);
    }
    payload == expected.as_slice()
}

/// Whether the durable claim receipt for `offer` joins a minted item instance (read-only).
async fn receipt_exists(clients: &SeamClients<'_>, offer: &[u8; 16]) -> TestResult<bool> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_returner_reward_claims c \
         JOIN game_item_instances i ON i.item_instance_id = c.item_instance_id \
         WHERE c.offer_id = encode($1, 'hex')::uuid",
    )
    .bind(offer.as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(count == 1)
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult<[u8; 16]> {
    let initial = state(client).await?;
    let inventory = client.inventory().cloned();
    match (initial.state, initial.id) {
        (1, Some(id)) => {
            let (status, _) = send(client, CAPABILITY, returner::CLAIM, &BOGUS).await?;
            let still_available = state(client).await?;
            if status != CommandStatus::Rejected
                || still_available.state != 1
                || still_available.id != Some(id)
                || receipt_exists(clients, &id).await?
                || client.inventory().cloned() != inventory
            {
                return Err("step=bogus-claim observed=entitlement-or-inventory-changed".into());
            }
            evidence("G17_CHECK step=history-entitlement status=PASS");
            let (status, reply) = send(client, CAPABILITY, returner::CLAIM, &id).await?;
            if status != CommandStatus::Accepted {
                return Err(format!(
                    "step=returner-claim observed=status:{status:?},payload:{reply:?}"
                )
                .into());
            }
            if !claim_reply_matches(&reply, 1, Some(id)) {
                return Err(format!("step=claim observed=reply:{reply:?}").into());
            }
            let after = state(client).await?;
            if after.state != 2 {
                return Err(format!("step=claim observed=state:{}", after.state).into());
            }
            let replay = accepted(client, CAPABILITY, returner::CLAIM, &id).await?;
            if !claim_reply_matches(&replay, 2, None) {
                return Err(format!("step=claim-replay observed=reply:{replay:?}").into());
            }
            if !receipt_exists(clients, &id).await? {
                return Err("step=claim observed=no-claim-receipt-with-item".into());
            }
            Ok(id)
        }
        (0, _) => {
            let (status, _) = send(client, CAPABILITY, returner::CLAIM, &BOGUS).await?;
            let after = state(client).await?;
            if status == CommandStatus::Accepted
                || after.state != 0
                || client.inventory().cloned() != inventory
            {
                return Err(format!(
                    "step=bogus-claim observed=status:{status:?}-state:{} needed=refusal-without-change",
                    after.state
                )
                .into());
            }
            Err("step=query observed=state-0-no-entitlement needed=returner-entitlement-after-12-month-inactivity".into())
        }
        (other, _) => Err(format!(
            "step=query observed=state-{other} needed=fresh-returner-entitlement"
        )
        .into()),
    }
}

async fn verify(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    data: &[u8; 16],
) -> TestResult {
    let after = state(client).await?;
    if after.state == 1 {
        return Err("step=relog observed=claimed-entitlement-offered-again".into());
    }
    if !receipt_exists(clients, data).await? {
        return Err("step=relog observed=no-claim-receipt-with-item".into());
    }
    Ok(())
}

/// Age the dedicated character's real terminal predecessor, keeping admission and end
/// evidence from the normal session owner. The next admission follows this session even
/// if this session itself has a training predecessor from a previous qualification run.
async fn seed_history(
    clients: &SeamClients<'_>,
    session: &[u8; 16],
    pending: Option<[u8; 16]>,
) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = sqlx::Connection::begin(&mut database).await?;
    sqlx::raw_sql(
        "SET LOCAL lock_timeout = '5s';
         SET CONSTRAINTS ALL IMMEDIATE;
         ALTER TABLE game_durability_reconnect_sessions DISABLE TRIGGER game_training_session_clock;
         ALTER TABLE game_character_session_ends DISABLE TRIGGER game_character_session_ends_immutable",
    )
    .execute(&mut *tx)
    .await?;
    if let Some(offer) = pending {
        // A retained offer can belong to an earlier session. Reset it so the claim
        // session creates its own offer and can still prove the state-2 transition.
        sqlx::query(
            "ALTER TABLE game_returner_reward_entitlements DISABLE TRIGGER returner_entitlement_immutable",
        )
        .execute(&mut *tx)
        .await?;
        let removed = sqlx::query(
            "DELETE FROM game_returner_reward_entitlements e
             WHERE e.offer_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid
               AND e.world_id=encode($3,'hex')::uuid AND e.account_id=$4::text::uuid
               AND NOT EXISTS (SELECT 1 FROM game_returner_reward_claims c WHERE c.offer_id=e.offer_id)",
        )
        .bind(offer.as_slice())
        .bind(clients.characters[0].as_slice())
        .bind(clients.world.as_slice())
        .bind(&clients.accounts[0])
        .execute(&mut *tx)
        .await?;
        if removed.rows_affected() != 1 {
            return Err("G17 expected one unclaimed fixture entitlement to reset".into());
        }
        sqlx::query(
            "ALTER TABLE game_returner_reward_entitlements ENABLE TRIGGER returner_entitlement_immutable",
        )
        .execute(&mut *tx)
        .await?;
    }
    let updated = sqlx::query(
        "WITH predecessor AS (
            UPDATE game_durability_reconnect_sessions
            SET training_created_at_us =
                extract(epoch FROM (clock_timestamp() - interval '13 months 1 day'))::bigint*1000000
            WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid
              AND world_id=encode($3,'hex')::uuid AND account_id=$4::text::uuid
              AND session_state=3
            RETURNING game_session_id,character_id,training_created_at_us
         )
         UPDATE game_character_session_ends e
         SET ended_at_us=p.training_created_at_us+1000000
         FROM predecessor p WHERE p.game_session_id=e.game_session_id AND p.character_id=e.character_id",
    )
    .bind(session.as_slice())
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(&clients.accounts[0])
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err("G17 expected one terminal predecessor and its real session end".into());
    }
    sqlx::raw_sql(
        "ALTER TABLE game_character_session_ends ENABLE TRIGGER game_character_session_ends_immutable;
         ALTER TABLE game_durability_reconnect_sessions ENABLE TRIGGER game_training_session_clock",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) fn setup_pending(initial: returner::Reply) -> TestResult<Option<[u8; 16]>> {
    match (initial.state, initial.id) {
        (0, None) => Ok(None),
        (1, Some(id)) => Ok(Some(id)),
        _ => Err("G17 setup expected no entitlement or an unclaimed entitlement".into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let accounts = [required("NODE_BOOT_G17_ACCOUNT")?];
    let characters = [full_row_g01::fixture_character(clients, &accounts[0]).await?];
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let mut predecessor = connect_character(clients, 0x7c, 0).await?;
    let previous = *predecessor.join_snapshot().game_session_id.as_bytes();
    let pending = setup_pending(state(&mut predecessor).await?)?;
    finish(&mut predecessor).await?;
    drop(predecessor);
    seed_history(clients, &previous, pending).await?;
    let mut client = connect_character(clients, 0x7c, 0).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let data = outcome?;
    closed?;
    let mut fresh = connect_character(clients, 0x7d, 0).await?;
    let checked = verdict::observe(verify(&mut fresh, clients, &data)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    checked?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn claim_reply_is_matched_exactly() {
        let mut claimed = vec![0x08, 1, 0x12, 16];
        claimed.extend_from_slice(&BOGUS);
        assert!(claim_reply_matches(&claimed, 1, Some(BOGUS)));
        assert!(!claim_reply_matches(&claimed, 2, None));
        assert!(claim_reply_matches(&[0x08, 2], 2, None));
        assert!(!claim_reply_matches(&[0x08, 3], 1, Some(BOGUS)));
    }

    #[test]
    fn bogus_identity_is_a_uuid_v7() {
        assert_eq!(BOGUS[6] >> 4, 7);
        assert_eq!(BOGUS[8] >> 6, 2);
        assert_eq!(returner::decode_claim(&BOGUS), Ok(BOGUS));
    }
}
