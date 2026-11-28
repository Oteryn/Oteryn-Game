//! W16 client replay capture: the player walks to the Summer 2026 NPC Wayland Smythers
//! (`TARGET_WALK`) and says `hi`, `danger`, `name`, `bye`. The driver asserts each server
//! disposition and catalogue reply and writes the raw `STATE_DOMAIN_NPC_CONVERSATION` delta
//! payloads (`Session::read_gated_delta`) that the session replay consumes
//! (`crates/session/src/tests/e2e_w16.rs`).
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;

const NPC: &str = "oteryn:npc.wayland_smythers";
const STEPS: [(&str, &str); 4] = [
    ("hi", "Wayland Smythers, in service of science"),
    ("danger", "humble messenger"),
    ("name", "Wayland Smythers"),
    ("bye", "do be careful"),
];

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W16_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    (text, needle): (&str, &str),
) -> TestResult<String> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step={text} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step={text} command status {:?}", reply.status).into());
    }
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step={text} result: {error:?}"))?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step={text} observed {disposition:?}").into());
    }
    let delta = loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence > reply.server_sequence {
            break delta;
        }
    };
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step={text} conversation: {error:?}"))?;
    if !conversation.lines.iter().any(|line| line.contains(needle)) {
        return Err(format!("step={text} lines {:?} lack {needle:?}", conversation.lines).into());
    }
    capture(&format!("w16-{text}-conversation.bin"), &delta.payload);
    let open = conversation.open.map_or_else(
        || "-".to_owned(),
        |(actor, _)| {
            let identity: String = actor.identity.iter().map(|byte| format!("{byte:02x}")).collect();
            format!("{identity}:{}", actor.generation)
        },
    );
    Ok(format!(
        "{text}\t{}\t{}\t{}\t{}\t{open}\t{}\n",
        wire::STATE_DOMAIN_NPC_CONVERSATION,
        delta.server_sequence,
        delta.base_revision,
        delta.new_revision,
        conversation.lines.join("\u{1f}")
    ))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == NPC)
        .ok_or("W16 client capture: Wayland Smythers absent from the bundle")?;
    let placement = npc.placements.first().ok_or("W16 client capture: no placement")?;
    let target = ActorPosition {
        x: i32::from(placement.x),
        y: i32::from(placement.y),
        floor: oteryn_protocol_oteryn::world_spatial::native_floor(i16::from(placement.floor)),
    };
    let world = full_e2ef7::qualification_world(clients)?;
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_W16_ACCOUNT").await?;
    let fixture_clients = fixture.clients(clients);
    let initial = &fixture_clients;
    let mut client = connect_character(initial, 0x89, 0).await?;
    super::full_row_w16::logout_w16(&mut client).await?;
    drop(client);
    let mut client = super::full_row_w16::approach(initial, world, 0x89, NPC, target).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entry| entry.kind == EntityKind::Npc && entry.position == target)
        })
        .map(|entry| entry.entity)
        .ok_or("W16 client capture: Wayland Smythers not visible")?;
    let mut expected = String::new();
    let mut failure = None;
    for step in STEPS {
        match talk(&mut client, actor, step).await {
            Ok(line) => expected.push_str(&line),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    super::full_row_w16::logout_w16(&mut client).await?;
    if let Some(error) = failure {
        return Err(error);
    }
    capture("w16-expected.txt", expected.as_bytes());
    Ok(format!("frames={}", STEPS.len()))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W16.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W16.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W16.client status=FAIL evidence=timeout"),
    }
}
