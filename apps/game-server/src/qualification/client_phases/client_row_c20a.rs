//! C20a client replay capture: query GeneralStats before and after a real lethal PvE encounter.

use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_character as wire};

pub(super) async fn run_packet(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let result = row_complete(capture_journey(clients, world, spawns))
        .await
        .map_err(|_| "C20a client capture timed out".into())
        .and_then(|result| result);
    match result {
        Ok(()) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C20a.client status=PASS evidence=general-stats-before-after-real-death",
        ),
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C20a.client status=FAIL evidence={error}"
        )),
    }
}

async fn stats(client: &mut DevClientSession) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[wire::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C20a GeneralStats query refused: {:?}", reply.status).into());
    }
    Ok(reply.payload)
}

fn progress(payload: &[u8]) -> TestResult<(u64, u32)> {
    if payload.len() != 42 || payload.first() != Some(&(wire::CharacterTab::GeneralStats as u8)) {
        return Err("C20a malformed GeneralStats reply".into());
    }
    Ok((
        u64::from_le_bytes(payload[1..9].try_into()?),
        u32::from_le_bytes(payload[9..13].try_into()?),
    ))
}

async fn capture_journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let mut client = connect_world(clients, 0xca).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let before_payload = stats(&mut client).await?;
        let before = progress(&before_payload)?;
        travel_to_spawn(&mut client, world, spawns, "oteryn:creature.poison_spider").await?;
        client.session_mut().attack_target(None).await?;
        client.take_events();
        let health = client
            .actor_vitals()
            .ok_or("C20a death entry vitals missing")?
            .health;
        let encounter = client.world_spatial().actor_position;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
        let mut damaged = false;
        loop {
            client.service_liveness(Duration::from_millis(50)).await?;
            for event in client.take_events() {
                if let SessionEvent::ActorVitals(delta) = event {
                    damaged |= delta.value.health < health;
                }
            }
            let position = client.world_spatial().actor_position;
            if damaged
                && position != encounter
                && client
                    .actor_vitals()
                    .is_some_and(|vitals| vitals.health > 0 && vitals.health == vitals.max_health)
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("C20a did not observe lethal damage and respawn".into());
            }
        }
        let after_payload = stats(&mut client).await?;
        let after = progress(&after_payload)?;
        if after.0 >= before.0 {
            return Err(
                format!("C20a death did not reduce experience: {before:?} -> {after:?}").into(),
            );
        }
        write_capture(&before_payload, &after_payload, before, after)?;
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

fn write_capture(before: &[u8], after: &[u8], old: (u64, u32), new: (u64, u32)) -> TestResult {
    let result: TestResult = (|| {
        let directory =
            std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("E2E_ALL_CAPTURE_DIR disappeared")?;
        let directory = Path::new(&directory);
        std::fs::write(directory.join("c20a-before.bin"), before)?;
        std::fs::write(directory.join("c20a-after.bin"), after)?;
        std::fs::write(
            directory.join("c20a-expected.txt"),
            format!("{} {}\n{} {}\n", old.0, old.1, new.0, new.1),
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C20a.client status=CAPTURE-WRITE-FAIL evidence={error}"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_rejects_short_or_wrong_tab() {
        assert!(progress(&[]).is_err());
        assert!(progress(&[0; 12]).is_err());
        assert!(progress(&[255; 42]).is_err());
    }
}
