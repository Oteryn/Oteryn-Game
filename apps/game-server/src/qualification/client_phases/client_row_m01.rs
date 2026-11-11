//! Capture the real capability-64 receipts consumed by the native M01 client adapter.
use super::*;
use super::{
    row_connect_character as connect_character,
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_protocol_oteryn::target_move::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = journey(clients).await {
        evidence(&format!(
            "e2e_all scenario=M01.client status=FAIL evidence={error}"
        ));
    }
}

async fn command(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let result = client
        .session_mut()
        .gated_command(
            CAPABILITY_TARGET_MOVE_V1,
            COMMAND_TYPE_TARGET_MOVE_INTENT,
            payload,
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("target-move command refused: {:?}", result.status).into());
    }
    Ok(result.payload)
}

fn disposition(payload: &[u8]) -> TestResult<&'static str> {
    match payload {
        [8, 1] => Ok("accepted"),
        [8, 2] => Ok("path-unavailable"),
        _ => Err("M01 invalid target-move disposition".into()),
    }
}

fn capture(name: &str, payload: &[u8]) -> TestResult<&'static str> {
    let outcome = disposition(payload)?;
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(outcome);
    };
    std::fs::write(
        std::path::Path::new(&root).join(format!("m01-{name}.bin")),
        payload,
    )?;

    Ok(outcome)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x6d).await?;
    let row_needs_logout_0 = true;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let start = tile(client.world_spatial().actor_position)?;
        let path = route(world, start, |at| at != start).ok_or("M01 capture movement lane")?;
        let direction = *path.first().ok_or("M01 capture empty route")?;
        let _ = client.step(direction).await?;
        let next = destination(
            world,
            tile(client.world_spatial().actor_position)?,
            direction,
        )
        .ok_or("M01 capture autowalk destination")?;
        let walk = encode_target_move_intent(TargetMoveIntent::Walk(ActorPosition {
            x: i32::from(next.x),
            y: i32::from(next.y),
            floor: -i16::from(next.floor),
        }))
        .map_err(|error| format!("M01 walk encode: {error:?}"))?;
        let autowalk = capture("autowalk", &command(&mut client, &walk).await?)?;
        let stop = capture("stop", &command(&mut client, &[16, 1]).await?)?;
        let turn = capture(
            "turn",
            &command(&mut client, &[26, 2, 8, StepDirection::North as u8]).await?,
        )?;

        let mut peer = connect_character(clients, 0x6e, 0).await?;
        let row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            peer.service_liveness(Duration::from_millis(25)).await?;
            client.service_liveness(Duration::from_millis(25)).await?;
            let entities = client.world_entities().ok_or("M01 capture entity view")?;
            let target = entities
                .others()
                .next()
                .ok_or("M01 capture follow target")?
                .entity;
            let encoded =
                oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&target))
                    .map_err(|error| format!("M01 follow encode: {error:?}"))?;
            let mut follow = vec![34, u8::try_from(encoded.len())?];
            follow.extend(encoded);
            let follow = capture("follow", &command(&mut client, &follow).await?)?;
            if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                std::fs::write(
                    std::path::Path::new(&root).join("m01-expected.txt"),
                    format!("autowalk={autowalk}\nstop={stop}\nturn={turn}\nfollow={follow}\n"),
                )?;
            }
            Ok(())
        })
    .await
    .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut peer)).await??;
        }
        row_result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_disposition_preserves_path_unavailable() -> TestResult {
        assert_eq!(disposition(&[8, 1])?, "accepted");
        assert_eq!(disposition(&[8, 2])?, "path-unavailable");
        assert!(disposition(&[8, 3]).is_err());
        assert!(disposition(&[]).is_err());
        Ok(())
    }
}
