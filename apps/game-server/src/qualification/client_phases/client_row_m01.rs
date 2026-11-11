//! Capture the real capability-64 receipts consumed by the native M01 client adapter.
use super::*;
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

fn capture(name: &str, payload: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(
        std::path::Path::new(&root).join(format!("m01-{name}.bin")),
        payload,
    )?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
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
    let row_result: TestResult<_> = async {
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
        capture("autowalk", &command(&mut client, &walk).await?)?;
        capture("stop", &command(&mut client, &[16, 1]).await?)?;
        capture(
            "turn",
            &command(&mut client, &[26, 2, 8, StepDirection::North as u8]).await?,
        )?;

        let mut peer = connect_character(clients, 0x6e, 0).await?;
        let row_needs_logout_1 = true;
        let row_result: TestResult<_> = async {
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
            capture("follow", &command(&mut client, &follow).await?)?;
            if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR")
                && let Err(error) = std::fs::write(
                    std::path::Path::new(&root).join("m01-expected.txt"),
                    "autowalk=accepted\nstop=accepted\nturn=accepted\nfollow=accepted\n",
                )
            {
                evidence(&format!(
                    "M01_CLIENT_CAPTURE_WRITE name=expected error={error}"
                ));
            }
            Ok(())
        }
        .await;
        if row_needs_logout_1 {
            row_logout(&mut peer).await?;
        }
        row_result
    }
    .await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}
