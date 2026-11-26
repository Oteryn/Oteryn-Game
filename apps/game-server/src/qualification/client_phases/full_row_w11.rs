//! W11: Soul War, taints, and Rotten Blood encounters.
//!
//! Concrete player scenario derived from the W11 audit/matrix row and the live endgame-entry,
//! taint-query, and quest-state owners:
//! 1. Admit the node-boot character and send boss command 133 with the taint projection opt-in;
//!    expect ACCEPTED plus a bounded Soul War/Rotten Blood taint snapshot.
//! 2. Send player boss-lever command 178 for Goshnar's Cruelty as an ordinary starter and
//!    require the typed level refusal before an instance can be entered.
//! 3. Send a malformed boss-lever command as the negative path; expect REJECTED with no payload,
//!    then repeat command 133 to prove that the refused command changed no taint state.
//! 4. Log out normally, obtain a fresh admission, and repeat command 133; expect the durable
//!    Soul War and Rotten Blood taint projection to equal the pre-logout snapshot.
//! 5. Prepare the dedicated challenger at the donor's level threshold, then read its level
//!    and derived vitals after ordinary admission before attempting the positive leg.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter as client_boss;
use oteryn_protocol_oteryn::boss_encounter as wire;

const SOURCE_GAP: &str = "crates/gameplay/dispatch/src/gameplay_transport/endgame_entry.rs:55";
const PROFILE: &str = "w11-soul-war";
const ENTRY_LEVEL: u32 = 250;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => (
            "FAIL",
            "step=journey observed=timeout waiting for server frames".into(),
        ),
    };
    evidence(&format!(
        "e2e_all scenario=W11.full status={status} seconds={:.3} evidence={}",
        started.elapsed().as_secs_f64(),
        compact(&detail),
    ));
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

async fn taints(client: &mut DevClientSession) -> TestResult<client_boss::Taints> {
    let request =
        client_boss::query_taints("").ok_or("taint query encoder refused empty cursor")?;
    let reply = command(client, request.kind, &request.payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=taint-query observed=status:{:?},payload:{:?}",
            reply.status, reply.payload
        )
        .into());
    }
    client_boss::decode(&reply.payload)
        .and_then(|snapshot| snapshot.taints)
        .ok_or_else(|| {
            format!(
                "step=taint-query observed=missing-or-invalid-delta:{:?}",
                reply.payload
            )
            .into()
        })
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xb1).await?;
    let before = taints(&mut client).await?;
    evidence(&format!(
        "W11_CHECK step=1 taints status=PASS observed={before:?}"
    ));

    let payload = wire::encode_boss_key("oteryn:creature.goshnar_s_cruelty")
        .map_err(|error| format!("Soul War boss-key encoding: {error:?}"))?;
    let entry = command(&mut client, wire::COMMAND_TYPE_BOSS_LEVER, &payload).await?;
    let entry_reply = wire::decode_boss_activity_reply(&entry.payload).map_err(|error| {
        format!(
            "step=soul-war-entry observed=status:{:?},payload:{:?},decode:{error:?}",
            entry.status, entry.payload
        )
    })?;
    evidence(&format!(
        "W11_CHECK step=2 soul-war-entry observed_status={:?} observed_reply={entry_reply:?}",
        entry.status
    ));

    let malformed = command(&mut client, wire::COMMAND_TYPE_BOSS_LEVER, &[10, 3, b'b']).await?;
    if malformed.status != CommandStatus::Rejected || !malformed.payload.is_empty() {
        return Err(format!(
            "step=invalid-lever-refusal observed=status:{:?},payload:{:?}",
            malformed.status, malformed.payload
        )
        .into());
    }
    let after_refusal = taints(&mut client).await?;
    if after_refusal != before {
        return Err(format!(
            "step=refusal-state-delta observed=before:{before:?},after:{after_refusal:?}"
        )
        .into());
    }
    evidence("W11_CHECK step=3 invalid-lever-refusal status=PASS state=unchanged");

    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xb2).await?;
    let retained = taints(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == old_session || retained != before {
        return Err(format!(
            "step=fresh-admission observed=same-session:{},before:{before:?},after:{retained:?}",
            fresh.join_snapshot().game_session_id == old_session
        )
        .into());
    }
    logout(&mut fresh).await?;
    evidence(&format!(
        "W11_CHECK step=4 fresh-admission status=PASS retained={retained:?}"
    ));

    if entry.status != CommandStatus::Rejected
        || entry_reply != wire::BossActivityReply::Refused(wire::BossActivityRefusal::Level)
    {
        return Err(format!(
            "step=starter-entry observed=status:{:?},reply:{entry_reply:?}",
            entry.status
        )
        .into());
    }
    let mut challenger = prepare_challenger(clients).await?;
    let entry = command(&mut challenger, wire::COMMAND_TYPE_BOSS_LEVER, &payload).await?;
    let reply = wire::decode_boss_activity_reply(&entry.payload).map_err(|error| {
        format!(
            "step=challenger-entry status={:?} decode={error:?}",
            entry.status
        )
    })?;
    evidence(&format!(
        "W11_CHECK step=6 challenger-entry observed_status={:?} observed_reply={reply:?}",
        entry.status
    ));
    match (entry.status, reply) {
        (CommandStatus::Accepted, wire::BossActivityReply::Accepted { .. }) => Err(
            "step=encounter-combat observed=activity-handoff;need=complete native boss combat, taint delta, chained fights, reward, and relog verification"
                .into(),
        ),
        (_, wire::BossActivityReply::Refused(reason)) => Err(format!(
            "step=challenger-entry observed=status:{:?},refusal:{reason:?};need=legitimate quest access and qualified boss instance;source={SOURCE_GAP}",
            entry.status
        )
        .into()),
        (status, reply) => Err(format!(
            "step=challenger-entry observed=status:{status:?},reply:{reply:?}",
        )
        .into()),
    }
}

async fn prepare_challenger(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W11_ACCOUNT").await?;
    let clients = dedicated.clients(clients);
    character_prep::declare_seeded(
        "W11.full",
        "level=250",
        "Crystal Soul War admission minimum",
    );
    let mut client = connect_character(&clients, 0xb3, 0).await?;
    let original = client.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(original.as_bytes()), "--profile", PROFILE.into()).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(&clients, 0xb4, 0).await?;
    let level = full_e2ef7::player_level(&mut client).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "knight",
        ENTRY_LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("W11 knight vitals formula absent")?;
    if client.join_snapshot().game_session_id == original
        || level != ENTRY_LEVEL
        || !client
            .actor_vitals()
            .is_some_and(|vitals| vitals.max_health == max_health && vitals.max_mana == max_mana)
    {
        return Err(format!(
            "step=challenger-stats observed=level:{level},vitals:{:?},same-session:{}",
            client.actor_vitals(),
            client.join_snapshot().game_session_id == original
        )
        .into());
    }
    evidence(&format!(
        "W11_CHECK step=5 status=PASS level={level} max_health={max_health} max_mana={max_mana}"
    ));
    Ok(client)
}

fn compact(detail: &str) -> String {
    detail
        .chars()
        .map(|character| {
            if character.is_whitespace() {
                '-'
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_detail_is_one_token() {
        assert_eq!(
            compact("step=entry observed=Level\nneed=instance"),
            "step=entry-observed=Level-need=instance"
        );
    }
}
