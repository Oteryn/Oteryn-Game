//! S16a client replay capture: Oressa's first-vocation dialogue as an ordinary player. The
//! driver says `hi`, `druid`, `yes` to Oressa with an already-chosen character, asserts each
//! server disposition and reply line, and writes the raw `STATE_DOMAIN_NPC_CONVERSATION` delta
//! payloads (`Session::read_gated_delta`) that the session replay consumes
//! (`crates/session/src/tests/e2e_s16a.rs`).
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("S16A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

struct Step {
    name: &'static str,
    text: &'static str,
    disposition: wire::NpcIntentDisposition,
    line: &'static str,
}

const STEPS: [Step; 3] = [
    Step {
        name: "hi",
        text: "hi",
        disposition: wire::NpcIntentDisposition::Ok,
        line: "Welcome. Ask about a vocation when you are ready to choose.",
    },
    Step {
        name: "druid",
        text: "druid",
        disposition: wire::NpcIntentDisposition::Ok,
        line: "Do you wish to become a druid? Say yes to confirm, or no to reconsider.",
    },
    Step {
        name: "yes",
        text: "yes",
        disposition: wire::NpcIntentDisposition::Rejected,
        line: "Reach level 8 before making your first vocation choice.",
    },
];

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    step: &Step,
    previous: &mut Option<u64>,
) -> TestResult<String> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: step.text.into(),
    })
    .map_err(|error| format!("step={} encode: {error:?}", step.name))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let reply = loop {
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_NPC_SERVICE_V1,
                wire::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("step={} command status {:?}", step.name, reply.status).into());
        }
        let disposition = wire::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("step={} result: {error:?}", step.name))?;
        if disposition == wire::NpcIntentDisposition::RateLimited
            && tokio::time::Instant::now() < deadline
        {
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
            continue;
        }
        if disposition != step.disposition {
            return Err(format!(
                "step={} expected {:?} observed {disposition:?}",
                step.name, step.disposition
            )
            .into());
        }
        break reply;
    };
    let delta = loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence > reply.server_sequence {
            break delta;
        }
    };
    if previous.is_some_and(|new| new != delta.base_revision) {
        return Err(format!(
            "step={} revision gap: base {} after {previous:?}",
            step.name, delta.base_revision
        )
        .into());
    }
    *previous = Some(delta.new_revision);
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step={} conversation: {error:?}", step.name))?;
    if conversation.open.map(|(actor, _)| actor) != Some(npc) || conversation.lines != [step.line] {
        return Err(format!("step={} different conversation {conversation:?}", step.name).into());
    }
    capture(&format!("s16a-{}-conversation.bin", step.name), &delta.payload);
    let identity: String = npc.identity.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "{}\t{}\t{}\t{}\t{}\t{identity}:{}\t{}\n",
        step.name,
        wire::STATE_DOMAIN_NPC_CONVERSATION,
        delta.server_sequence,
        delta.base_revision,
        delta.new_revision,
        npc.generation,
        conversation.lines.join("\u{1f}")
    ))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x41, 0).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let oressa = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.oressa")
        .ok_or("S16a client capture: Oressa absent")?;
    let path = route(world, start, |at| {
        oressa.placements.iter().any(|place| {
            place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })
    })
    .ok_or("S16a client capture: Oressa unreachable")?;
    walk(&mut client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && oressa.placements.iter().any(|place| {
                        entity.position.x == i32::from(place.x)
                            && entity.position.y == i32::from(place.y)
                            && entity.position.floor == -i16::from(place.floor)
                    })
            })
        })
        .map(|entity| entity.entity)
        .ok_or("S16a client capture: Oressa not visible")?;
    let mut expected = String::new();
    let mut previous = None;
    let mut failure = None;
    for step in &STEPS {
        match talk(&mut client, actor, step, &mut previous).await {
            Ok(line) => expected.push_str(&line),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    if let Some(back) = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == start
    }) && let Err(error) = walk(&mut client, world, &back).await
    {
        evidence(&format!("S16A_CLIENT_RETURN_WALK_ERROR error={error}"));
    }
    let _ = client.session_mut().logout().await;
    if let Some(error) = failure {
        return Err(error);
    }
    capture("s16a-expected.txt", expected.as_bytes());
    Ok(format!("frames={}", STEPS.len()))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S16a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S16a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S16a.client status=FAIL evidence=timeout"),
    }
}
