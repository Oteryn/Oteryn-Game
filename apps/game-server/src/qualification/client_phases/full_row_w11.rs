//! W11 live qualification: both pinned boss chains, settlement and fresh admission.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter as client_boss;
use oteryn_protocol_oteryn::boss_encounter as wire;

#[path = "full_row_w11_journey.rs"]
mod live;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(3600),
        verdict::observe(journey(clients)),
    )
    .await;
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
    let rewards = live::complete(clients, &mut client).await?;
    let completed = taints(&mut client).await?;
    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xb2).await?;
    let retained = taints(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == old_session || retained != completed {
        return Err(format!(
            "step=fresh-admission observed=same-session:{},before:{completed:?},after:{retained:?}",
            fresh.join_snapshot().game_session_id == old_session
        )
        .into());
    }
    live::verify_rewards(clients, &mut fresh, &rewards).await?;
    logout(&mut fresh).await?;
    Ok("all pinned levers, native victories, one-time grants and relog verified".into())
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
