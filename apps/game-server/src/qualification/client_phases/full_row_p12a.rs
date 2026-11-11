//! P12a concrete player scenario, derived from the matrix/re-audit and the
//! `character_title` durability owner:
//! 1. Send `QuerySelf`; require an accepted, well-formed character-info projection and retain
//!    its earned titles and selected title.
//! 2. Send `Unlock` for a title whose qualification was earned by the packet's ordinary player
//!    actions; require the correlated `[2, title]` reply, then query and require the earned set.
//! 3. Send `Select(title)`; require an accepted revision reply, then query and require the
//!    selected-title state delta.
//! 4. Send `Select` for a title not earned by the character; require rejection and query again
//!    to prove that the earned set and selected title did not change.
//! 5. Send `Unlock(Reigning Drome Champion)` to exercise the expiring-title path. If accepted,
//!    select it and require it in the projection; if the playable server cannot establish the
//!    preceding-rota top-five fact, report the observed refusal and the missing server path.
//! 6. Send ordinary logout, make a fresh admission, and query again; require a different game
//!    session and the same durable earned/selected state established by steps 2-4.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::character_info::{
    CAPABILITY_CHARACTER_INFO_V1, COMMAND_TYPE_CHARACTER_INFO, CharacterInfoOperation,
    CharacterTitle, PageRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Titles {
    earned: Vec<CharacterTitle>,
    selected: Option<CharacterTitle>,
}

async fn command(
    client: &mut DevClientSession,
    operation: CharacterInfoOperation,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    let payload = encode_operation(operation);
    Ok(client
        .session_mut()
        .gated_command(
            CAPABILITY_CHARACTER_INFO_V1,
            COMMAND_TYPE_CHARACTER_INFO,
            &payload,
        )
        .await?)
}

// The dev client exposes the generic native command adapter but no character-info helper.
// Keep this packet-local encoder byte-for-byte aligned with protocol-oteryn's operation shape.
fn encode_operation(operation: CharacterInfoOperation) -> Vec<u8> {
    match operation {
        CharacterInfoOperation::QueryPremium => vec![5],
        CharacterInfoOperation::QuerySelf(page) => {
            let [a, b] = page.offset.to_le_bytes();
            vec![0, a, b, page.limit]
        }
        CharacterInfoOperation::QueryVisible { character, page } => {
            let mut bytes = vec![1];
            bytes.extend_from_slice(&character);
            bytes.extend_from_slice(&page.offset.to_le_bytes());
            bytes.push(page.limit);
            bytes
        }
        CharacterInfoOperation::Unlock(title) => vec![2, title.id()],
        CharacterInfoOperation::Select(title) => {
            vec![3, title.map_or(0, CharacterTitle::id)]
        }
        CharacterInfoOperation::LookVisible { character } => {
            let mut bytes = vec![4];
            bytes.extend_from_slice(&character);
            bytes
        }
    }
}

fn titles(payload: &[u8]) -> TestResult<Titles> {
    if payload.first() != Some(&0) || payload.len() < 71 {
        return Err(format!("step query: malformed character-info reply len={}", payload.len()).into());
    }
    let count = usize::from(payload[69]);
    if count > oteryn_protocol_oteryn::character_info::TITLE_COUNT
        || payload.len() != 71 + count
    {
        return Err(format!("step query: invalid title projection count={count} len={}", payload.len()).into());
    }
    let mut earned = Vec::with_capacity(count);
    for raw in &payload[70..70 + count] {
        let title = CharacterTitle::from_id(*raw)
            .ok_or_else(|| format!("step query: unknown earned title id={raw}"))?;
        if earned.contains(&title) {
            return Err(format!("step query: duplicate earned title id={raw}").into());
        }
        earned.push(title);
    }
    let selected = match payload[70 + count] {
        0 => None,
        raw => Some(
            CharacterTitle::from_id(raw)
                .ok_or_else(|| format!("step query: unknown selected title id={raw}"))?,
        ),
    };
    if selected.is_some_and(|selected| !earned.contains(&selected)) {
        return Err("step query: selected title absent from earned set".into());
    }
    Ok(Titles { earned, selected })
}

async fn query(client: &mut DevClientSession) -> TestResult<Titles> {
    let page = PageRequest::new(0, 1).ok_or("step query: invalid page")?;
    let reply = command(client, CharacterInfoOperation::QuerySelf(page)).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step query: observed status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    titles(&reply.payload)
}

async fn logout(client: &mut RowClient) -> TestResult {
    let reply = client.row_logout().await?;
    if reply.status != CommandStatus::Accepted
        || reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("step logout: observed reply={reply:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0xc1).await?;
    let initial_session = client.join_snapshot().game_session_id;
    let initial = query(&mut client).await?;
    let earned = initial
        .earned
        .first()
        .copied()
        .ok_or("step 1 query: no title earned through the preceding ordinary player journeys")?;

    let unlock = command(&mut client, CharacterInfoOperation::Unlock(earned)).await?;
    if unlock.status != CommandStatus::Accepted || unlock.payload != [2, earned.id()] {
        return Err(format!("step 2 unlock {}: observed reply={unlock:?}", earned.name()).into());
    }
    let after_unlock = query(&mut client).await?;
    if !after_unlock.earned.contains(&earned) {
        return Err(format!("step 2 unlock {}: accepted reply missing earned-state delta", earned.name()).into());
    }

    let select = command(&mut client, CharacterInfoOperation::Select(Some(earned))).await?;
    if select.status != CommandStatus::Accepted || select.payload.len() != 9 || select.payload[0] != 3 {
        return Err(format!("step 3 select {}: observed reply={select:?}", earned.name()).into());
    }
    let selected = query(&mut client).await?;
    if selected.selected != Some(earned) {
        return Err(format!("step 3 select {}: projection={selected:?}", earned.name()).into());
    }

    let refused = CharacterTitle::ALL
        .into_iter()
        .find(|title| !selected.earned.contains(title))
        .ok_or("step 4 refusal: character unexpectedly earned the complete title catalogue")?;
    let rejection = command(&mut client, CharacterInfoOperation::Select(Some(refused))).await?;
    if rejection.status != CommandStatus::Rejected {
        return Err(format!("step 4 select-unearned {}: observed reply={rejection:?}", refused.name()).into());
    }
    if query(&mut client).await? != selected {
        return Err("step 4 select-unearned: refusal changed title state".into());
    }

    let drome = CharacterTitle::ReigningDromeChampion;
    let expiry = command(&mut client, CharacterInfoOperation::Unlock(drome)).await?;
    let expiry_refusal = (expiry.status == CommandStatus::Rejected).then(|| {
        format!(
            "step 5 expiring unlock: observed status={:?} payload={:?}; server needs a player-reachable preceding-rota top-five path",
            expiry.status, expiry.payload
        )
    });
    if expiry_refusal.is_none()
        && (expiry.status != CommandStatus::Accepted || expiry.payload != [2, drome.id()])
    {
        return Err(format!("step 5 expiring unlock: observed reply={expiry:?}").into());
    }
    if expiry_refusal.is_none() {
        let selected_drome =
            command(&mut client, CharacterInfoOperation::Select(Some(drome))).await?;
        if selected_drome.status != CommandStatus::Accepted
            || query(&mut client).await?.selected != Some(drome)
        {
            return Err(
                format!("step 5 expiring select: observed reply={selected_drome:?}").into(),
            );
        }
    }

    logout(&mut client).await?;
    drop(client);
    let mut fresh = row_connect_world(clients, 0xc2).await?;
    if fresh.join_snapshot().game_session_id == initial_session {
        return Err("step 6 fresh admission reused prior game session".into());
    }
    let mut durable = query(&mut fresh).await?;
    if expiry_refusal.is_some() && durable != selected {
        return Err(format!("step 6 durable title state mismatch: before={selected:?} after={durable:?}").into());
    }
    if let Some(refusal) = expiry_refusal {
        logout(&mut fresh).await?;
        return Err(refusal.into());
    }
    if durable.selected != Some(drome) || !durable.earned.contains(&drome) {
        return Err(format!(
            "step 6 durable expiring-title mismatch: after={durable:?}"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    while durable.selected == Some(drome) || durable.earned.contains(&drome) {
        if tokio::time::Instant::now() >= deadline {
            logout(&mut fresh).await?;
            return Err("step 5 expiry: title remained earned/selected for the 120s observation window; server needs an observable player-driven rotation boundary".into());
        }
        fresh.service_liveness(Duration::from_secs(1)).await?;
        fresh.take_events();
        durable = query(&mut fresh).await?;
    }
    logout(&mut fresh).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(180), journey(clients)).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=P12a.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P12a.full status=FAIL seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            error.to_string().replace(' ', "-")
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=P12a.full status=FAIL seconds={:.3} evidence=step-timeout-180s",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_projection_requires_correlated_selected_earned_state() -> TestResult {
        let mut payload = vec![0; 71];
        payload[69] = 1;
        payload.insert(70, CharacterTitle::AdeptArmorer.id());
        payload[71] = CharacterTitle::AdeptArmorer.id();
        assert_eq!(titles(&payload)?.selected, Some(CharacterTitle::AdeptArmorer));
        payload[71] = CharacterTitle::Globetrotter.id();
        assert!(titles(&payload).is_err());
        Ok(())
    }
}
