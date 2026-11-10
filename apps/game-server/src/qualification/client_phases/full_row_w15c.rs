//! W15c Killing in the Name Of (Grizzly Adams), derived from the NPC conversation owner
//! (`gameplay_transport/npc_command/grizzly.rs`) and the quest rules (`quest/grizzly_adams/rules.rs`).
//!
//! 1. Walk to the authored Grizzly Adams placement, say `hi`, then `join`; assert the server's
//!    reply lines (the durable join) and `points`.
//! 2. Ask for the lowest-level task by name; assert the exact reply: a start offer (confirmed with
//!    `yes`, then `status` must list it), or the exact level/premium refusal.
//! 3. Log out, obtain a fresh admission, walk back and say `hi`; the durable join must answer
//!    `Welcome back`, and an accepted task must still be listed by `status`.
//! 4. PASS only when a task was started and survived the fresh admission; a refusal reports FAIL
//!    naming the precondition. Never writes state outside the real conversation.
use super::*;
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::npc_service as wire;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const NPC: &str = "oteryn:npc.grizzly_adams";
const REFUSALS: [&str; 2] = [
    "This task requires Premium.",
    "That task is outside your level range.",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(150), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(Outcome::Started(task))) => (
            "PASS",
            format!(
                "task-started-and-durable-after-fresh-admission:{}",
                compact(&task)
            ),
        ),
        Ok(Ok(Outcome::Refused(line))) => (
            "FAIL",
            format!(
                "needed:character-eligible-for-a-task(level-and-premium);join-durable;refusal={}",
                compact(&line)
            ),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W15c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

enum Outcome {
    Started(String),
    Refused(String),
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(300)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

/// How the server answered a task request.
#[derive(Debug, PartialEq, Eq)]
enum TaskReply {
    Offer,
    Refusal,
    Other,
}

fn classify(line: &str) -> TaskReply {
    if line.starts_with("Hunt ") {
        TaskReply::Offer
    } else if REFUSALS.contains(&line) {
        TaskReply::Refusal
    } else {
        TaskReply::Other
    }
}

/// The catalogue's lowest-level task name; the server matches the typed text case-insensitively.
fn first_task() -> TestResult<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalogue: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/quests/killing-in-the-name-of.json"),
    )?)?;
    catalogue["tasks"]
        .as_array()
        .ok_or("step=catalogue observed=no-tasks")?
        .iter()
        .filter_map(|task| Some((task["minimum_level"].as_u64()?, task["name"].as_str()?)))
        .min()
        .map(|(_, name)| name.to_owned())
        .ok_or_else(|| "step=catalogue observed=empty".into())
}

async fn reach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
) -> TestResult<EntityRef> {
    let metadata = oteryn_world_bundle::bundle::visit(bytes, |_, _| Ok(()))?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == NPC)
        .ok_or("step=reach observed=no-grizzly-adams-placement-in-the-bundle")?;
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        npc.placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or("step=reach observed=no-walk-route-from-spawn-to-grizzly-adams")?;
    walk(client, world, &path).await?;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| "step=reach observed=grizzly-adams-not-visible-after-walk".into())
}

/// One real talk intent; returns the reply lines the server addressed to this character.
async fn say(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    client.take_events();
    let reply = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=say-{text} observed=disposition-{reply:?}").into());
    }
    let mut lines = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        for event in client.take_events() {
            if let SessionEvent::NpcConversation(delta) = event {
                lines.extend(delta.value.lines);
            }
        }
        if !lines.is_empty() || tokio::time::Instant::now() >= deadline {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    if lines.is_empty() {
        return Err(format!("step=say-{text} observed=no-reply-line").into());
    }
    Ok(lines)
}

fn expect(lines: &[String], needle: &str, step: &str) -> TestResult {
    if lines.iter().any(|line| line.contains(needle)) {
        Ok(())
    } else {
        Err(format!(
            "step={step} observed={} expected={needle}",
            compact(&lines.join("|"))
        )
        .into())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Outcome> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let task = first_task()?;
    let mut client = connect_world(clients, 0xe5).await?;
    let actor = reach(&mut client, world, &bytes).await?;
    expect(&say(&mut client, actor, "hi").await?, "join", "hi")?;
    expect(&say(&mut client, actor, "join").await?, "Welcome", "join")?;
    expect(
        &say(&mut client, actor, "points").await?,
        "Paw and Fur points",
        "points",
    )?;
    let reply = say(&mut client, actor, &task).await?;
    let line = reply.first().cloned().unwrap_or_default();
    let started = match classify(&line) {
        TaskReply::Offer => {
            expect(
                &say(&mut client, actor, "yes").await?,
                "Happy hunting",
                "confirm",
            )?;
            expect(&say(&mut client, actor, "status").await?, &task, "status")?;
            true
        }
        TaskReply::Refusal => false,
        TaskReply::Other => {
            return Err(format!("step=task observed=unexpected-reply:{}", compact(&line)).into());
        }
    };
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xe5).await?;
    let actor = reach(&mut client, world, &bytes).await?;
    expect(
        &say(&mut client, actor, "hi").await?,
        "Welcome back",
        "durable-join",
    )?;
    if started {
        expect(
            &say(&mut client, actor, "status").await?,
            &task,
            "durable-task",
        )?;
    }
    logout(&mut client).await?;
    Ok(if started {
        Outcome::Started(task)
    } else {
        Outcome::Refused(line)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_are_classified_by_their_exact_text() {
        assert_eq!(
            classify("Hunt 300 Crocodiles. Do you want to start this task? Say yes or no."),
            TaskReply::Offer
        );
        assert_eq!(classify("This task requires Premium."), TaskReply::Refusal);
        assert_eq!(
            classify("That task is outside your level range."),
            TaskReply::Refusal
        );
        assert_eq!(classify("Welcome"), TaskReply::Other);
    }

    #[test]
    fn tokens_are_stable() {
        assert_eq!(compact("a  b"), "a-b");
    }
}
