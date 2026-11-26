//! P15 client replay capture: the real exercise-training replies (STATUS, refused START, running
//! START, STOP) of a legitimately carried exercise weapon and an authored exercise dummy. The
//! consumer is `apply` in `apps/client/src/features/0054-training/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{item_view::ItemHandle, loop_training as wire};

use super::full_row_p15::{
    DummyView, carried_weapon, dummy_distance, dummy_positions, exercise_character, walk_to_probe,
};
use oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;
use std::io::Write;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::File::create_new(root.join(name))?.write_all(bytes)?;
    Ok(())
}

fn take_varint(bytes: &[u8], cursor: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let byte = *bytes.get(*cursor)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return None;
        }
        value |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Some(value);
        }
    }
    None
}

fn push_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Decode a training reply exactly as the wire encoder lays it out (used by P14 and P15).
pub(super) fn training_reply(bytes: &[u8]) -> Option<wire::TrainingResult> {
    let mut values = [None; 4];
    let mut cursor = 0;
    while cursor < bytes.len() {
        let tag = take_varint(bytes, &mut cursor)?;
        if !matches!(tag, 8 | 16 | 24 | 32) {
            return None;
        }
        let slot = &mut values[usize::try_from(tag / 8 - 1).ok()?];
        if slot.is_some() {
            return None;
        }
        *slot = Some(take_varint(bytes, &mut cursor)?);
    }
    let result = wire::TrainingResult {
        disposition: u8::try_from(values[0]?).ok()?,
        revision: values[1]?,
        pool_seconds: u32::try_from(values[2]?).ok()?,
        charges: u32::try_from(values[3]?).ok()?,
    };
    (wire::encode_training_result(result).ok()?.as_slice() == bytes).then_some(result)
}

async fn exercise(
    client: &mut DevClientSession,
    intent: wire::ExerciseIntent,
    expected_status: CommandStatus,
) -> TestResult<(Vec<u8>, wire::TrainingResult)> {
    let mut payload = vec![8];
    match intent {
        wire::ExerciseIntent::Start { weapon, dummy } => {
            payload.extend([1, 16]);
            push_varint(&mut payload, weapon.get());
            payload.push(24);
            push_varint(&mut payload, dummy.get());
        }
        wire::ExerciseIntent::Status => payload.push(2),
        wire::ExerciseIntent::Stop => payload.push(3),
    }
    let reply: GatedCommandResult = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TRAINING_V1,
            wire::COMMAND_TYPE_EXERCISE_TRAINING,
            &payload,
        )
        .await?;
    if reply.status != expected_status {
        return Err(format!("exercise command status={:?}", reply.status).into());
    }
    let decoded = training_reply(&reply.payload).ok_or("malformed training reply")?;
    let captured = serde_json::to_vec(&(
        reply.command_id,
        reply.server_sequence,
        reply.status as u32,
        reply.payload,
    ))?;
    Ok((captured, decoded))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use CommandStatus::{Accepted, Rejected};

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P15 capture directory missing")?;
    let root = Path::new(&root);
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_P15_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let mut client = exercise_character(clients, "P15.client", 0x41).await?;
    let (status_bytes, status) =
        exercise(&mut client, wire::ExerciseIntent::Status, Accepted).await?;
    if status.disposition != 2 || status.revision == 0 {
        return Err(format!("STATUS reply={status:?}").into());
    }
    capture(root, "p15-status.json", &status_bytes)?;

    let missing = ItemHandle::new(u64::MAX).ok_or("nonzero refusal handle")?;
    let (refused_bytes, refused) = exercise(
        &mut client,
        wire::ExerciseIntent::Start {
            weapon: missing,
            dummy: missing,
        },
        Rejected,
    )
    .await?;
    if refused != wire::TrainingResult::REFUSED {
        return Err(format!("invalid START reply={refused:?}").into());
    }
    capture(root, "p15-refused.json", &refused_bytes)?;

    let (weapon, _, _) = carried_weapon(&client)?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let positions = dummy_positions(&bytes)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        positions
            .iter()
            .any(|target| dummy_distance(at, *target) == Some(1))
    })
    .ok_or("no player-walkable route to an authored exercise dummy")?;
    let nearby = path.iter().try_fold(from, |at, &direction| {
        destination(world, at, direction).ok_or("planned exercise route has no landing")
    })?;
    let target = positions
        .into_iter()
        .find(|&target| dummy_distance(nearby, target) == Some(1))
        .ok_or("planned destination has no adjacent exercise dummy")?;
    let mut view = DummyView::admitted(&mut client, target)?;
    walk_to_probe(&mut client, world, &path, &mut view).await?;
    let dummy = view.handle(&mut client).await?;
    let (running_bytes, running) = exercise(
        &mut client,
        wire::ExerciseIntent::Start { weapon, dummy },
        Accepted,
    )
    .await?;
    if running.disposition != 1 || running.revision == 0 || running.charges == 0 {
        return Err(format!("START reply={running:?}").into());
    }
    capture(root, "p15-running.json", &running_bytes)?;
    let (stopped_bytes, stopped) =
        exercise(&mut client, wire::ExerciseIntent::Stop, Accepted).await?;
    if stopped.disposition != 2 || stopped.charges > running.charges {
        return Err(format!("STOP reply={stopped:?} after {running:?}").into());
    }
    capture(root, "p15-stopped.json", &stopped_bytes)?;
    let _ = client.session_mut().logout().await?;
    let expected = format!(
        "status_disposition={}\nstatus_charges={}\nrefused_disposition={}\nrunning_disposition={}\nrunning_charges={}\nstopped_disposition={}\nstopped_charges={}\n",
        status.disposition,
        status.charges,
        refused.disposition,
        running.disposition,
        running.charges,
        stopped.disposition,
        stopped.charges
    );
    capture(root, "p15-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P15.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P15.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P15.client status=FAIL evidence=timeout"),
    }
}
