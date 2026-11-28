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
        return Err(format!(
            "step query: malformed character-info reply len={}",
            payload.len()
        )
        .into());
    }
    let count = usize::from(payload[69]);
    if count > oteryn_protocol_oteryn::character_info::TITLE_COUNT || payload.len() != 71 + count {
        return Err(format!(
            "step query: invalid title projection count={count} len={}",
            payload.len()
        )
        .into());
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
        return Err(format!(
            "step query: session={} reply={reply:?}",
            uuid_text(client.join_snapshot().game_session_id.as_bytes())
        )
        .into());
    }
    titles(&reply.payload)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.status != CommandStatus::Accepted
        || reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("step logout: observed reply={reply:?}").into());
    }
    Ok(())
}

fn inert_refusal(status: CommandStatus, payload: &[u8], before: &Titles, after: &Titles) -> bool {
    status == CommandStatus::Rejected && payload.is_empty() && before == after
}

async fn prepare_title_inputs(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "P12a.full",
        "level-100,preceding-drome-rank-1",
        "ordinary title qualification and a completed rotation exceed the live row budget",
    );
    let client = character_prep::Seeded {
        profile: "explorer",
        reason: "drive ordinary title selection at level 100",
    }
    .connect(clients, 0xc1, 1)
    .await?;
    let character = clients.characters.get(1).ok_or("P12a character absent")?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut transaction = database.begin().await?;
    let level: i64 = sqlx::query_scalar(
        "SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_one(&mut *transaction)
    .await?;
    if level != 100 {
        return Err(format!("P12a persisted level={level}, expected 100").into());
    }
    let rotation: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp()) / (14 * 24 * 3600))::bigint - 1",
    )
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO game_tibiadrome_scores(world_id,rotation,character_id,level,experience)
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,20,1000)",
    )
    .bind(clients.world.as_slice())
    .bind(rotation)
    .bind(character.as_slice())
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO game_tibiadrome_awards(world_id,rotation,character_id,rank,potions)
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,3)",
    )
    .bind(clients.world.as_slice())
    .bind(rotation)
    .bind(character.as_slice())
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    let rank: i64 = sqlx::query_scalar(
        "SELECT rank FROM game_tibiadrome_awards WHERE world_id=encode($1,'hex')::uuid
         AND rotation=$2 AND character_id=encode($3,'hex')::uuid",
    )
    .bind(clients.world.as_slice())
    .bind(rotation)
    .bind(character.as_slice())
    .fetch_one(&mut database)
    .await?;
    if rank != 1 {
        return Err(format!("P12a persisted Drome rank={rank}, expected 1").into());
    }
    evidence(&format!("P12A_FIXTURE level={level} previous_rotation={rotation} rank={rank}"));
    Ok(client)
}

async fn rotation_boundary(client: &mut DevClientSession) -> TestResult<u32> {
    use oteryn_protocol_oteryn::arena::{
        ArenaAction, CAPABILITY_ARENA_V1, COMMAND_TYPE_ARENA_MATCH, DromeAction, DromeResult,
        encode_match_request,
    };
    let payload = encode_match_request(ArenaAction::Drome(DromeAction::Status))
        .map_err(|error| format!("Drome schedule request: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_ARENA_V1, COMMAND_TYPE_ARENA_MATCH, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Drome schedule refused: {reply:?}").into());
    }
    let status = DromeResult::decode(&reply.payload)
        .map_err(|error| format!("Drome schedule decode: {error:?}, reply={reply:?}"))?;
    if status.disposition != 1 || status.rotation_ends == 0 {
        return Err(format!("Drome schedule unavailable: {reply:?}").into());
    }
    Ok(status.rotation_ends)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = prepare_title_inputs(clients).await?;
    let initial_session = client.join_snapshot().game_session_id;
    let initial = query(&mut client).await?;
    let earned = CharacterTitle::Trolltrasher;
    if !initial.earned.contains(&earned) || initial.selected == Some(earned) {
        return Err(format!("P12a unexpected initial ordinary title state {initial:?}").into());
    }

    let unlock = command(&mut client, CharacterInfoOperation::Unlock(earned)).await?;
    if unlock.status != CommandStatus::Accepted || unlock.payload != [2, earned.id()] {
        return Err(format!("step 2 unlock {}: observed reply={unlock:?}", earned.name()).into());
    }
    let after_unlock = query(&mut client).await?;
    if !after_unlock.earned.contains(&earned) {
        return Err(format!(
            "step 2 unlock {}: accepted reply missing earned-state delta",
            earned.name()
        )
        .into());
    }

    let select = command(&mut client, CharacterInfoOperation::Select(Some(earned))).await?;
    if select.status != CommandStatus::Accepted
        || select.payload.len() != 9
        || select.payload[0] != 3
    {
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
    let retained = query(&mut client).await?;
    if !inert_refusal(rejection.status, &rejection.payload, &selected, &retained) {
        return Err(format!(
            "step=select-unearned title={} observed={rejection:?} retained={retained:?}",
            refused.name()
        )
        .into());
    }

    let drome = CharacterTitle::ReigningDromeChampion;
    let expiry = command(&mut client, CharacterInfoOperation::Unlock(drome)).await?;
    let expiry_refusal = (expiry.status == CommandStatus::Rejected).then(|| {
        format!(
            "step 5 expiring unlock: observed status={:?} payload={:?}; need=preceding-rota-top-five; source:crates/gameplay/durability/src/durability/character_title.rs:155",
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
    let mut fresh = connect_world(clients, 0xc2).await?;
    if fresh.join_snapshot().game_session_id == initial_session {
        return Err("step 6 fresh admission reused prior game session".into());
    }
    let durable = query(&mut fresh).await?;
    if expiry_refusal.is_some() && durable != selected {
        return Err(format!(
            "step 6 durable title state mismatch: before={selected:?} after={durable:?}"
        )
        .into());
    }
    if let Some(refusal) = expiry_refusal {
        logout(&mut fresh).await?;
        return Err(refusal.into());
    }
    if durable.selected != Some(drome) || !durable.earned.contains(&drome) {
        return Err(format!("step 6 durable expiring-title mismatch: after={durable:?}").into());
    }
    let boundary = rotation_boundary(&mut fresh).await?;
    evidence(&format!("P12A_ROTATION_WAIT server_rotation_ends={boundary} limit_seconds=120"));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        if tokio::time::Instant::now() >= deadline {
            logout(&mut fresh).await?;
            return Err(format!(
                "step 5 expiry: server rotation boundary {boundary} did not advance within 120s"
            )
            .into());
        }
        fresh.service_liveness(Duration::from_secs(1)).await?;
        fresh.take_events();
        let next = rotation_boundary(&mut fresh).await?;
        if next > boundary {
            evidence(&format!("P12A_ROTATION_ADVANCED previous_end={boundary} next_end={next}"));
            break;
        }
    }
    let expired = query(&mut fresh).await?;
    if expired.selected.is_some() || expired.earned.contains(&drome) {
        return Err(format!("Drome title survived rotation: {expired:?}").into());
    }
    logout(&mut fresh).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step=title-journey observed=180s-timeout".into()),
    };
    if let Some(error) = &failure {
        evidence(&format!("P12A_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=P12a.full status={} seconds={:.3}",
        if failure.is_none() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_cannot_carry_a_title_receipt() {
        let empty = Titles {
            earned: Vec::new(),
            selected: None,
        };
        assert!(inert_refusal(CommandStatus::Rejected, &[], &empty, &empty));
        assert!(!inert_refusal(
            CommandStatus::Rejected,
            &[3],
            &empty,
            &empty
        ));
    }

    #[test]
    fn title_projection_requires_correlated_selected_earned_state() -> TestResult {
        let mut payload = vec![0; 71];
        payload[69] = 1;
        payload.insert(70, CharacterTitle::AdeptArmorer.id());
        payload[71] = CharacterTitle::AdeptArmorer.id();
        assert_eq!(
            titles(&payload)?.selected,
            Some(CharacterTitle::AdeptArmorer)
        );
        payload[71] = CharacterTitle::Globetrotter.id();
        assert!(titles(&payload).is_err());
        Ok(())
    }
}
