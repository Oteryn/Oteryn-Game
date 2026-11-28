//! W01a records the original inbound session frames while observing Rat chase,
//! damage and flight on the live client.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityRef, SessionEvent, WorldSpatialEntity,
};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};
use sha2::{Digest, Sha256};

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    std::fs::File::create_new(root.join(name))
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|error| format!("W01a capture file={name}: {error}"))?;
    Ok(())
}

fn distance(a: ActorPosition, b: ActorPosition) -> u32 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

fn closes_distance(before: ActorPosition, after: ActorPosition, player: ActorPosition) -> bool {
    before.floor == after.floor
        && after.floor == player.floor
        && distance(after, player) < distance(before, player)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn text_of(entity: Option<&WorldSpatialEntity>) -> String {
    let Some(entity) = entity else {
        return "absent".into();
    };
    let health = match entity.detail {
        EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } => health_percent.to_string(),
        _ => "-".into(),
    };
    format!(
        "{},{},{}|{health}",
        entity.position.x, entity.position.y, entity.position.floor
    )
}

struct Segment {
    name: &'static str,
    target: EntityRef,
    initial_sequence: u64,
    initial: String,
    trace: Vec<String>,
    target_position: Option<ActorPosition>,
    closing_step: Option<(ActorPosition, ActorPosition, ActorPosition)>,
}

impl Segment {
    fn begin(client: &DevClientSession, name: &'static str, target: EntityRef) -> TestResult<Self> {
        let view = client.world_entities().ok_or("entity view absent")?;
        Ok(Self {
            name,
            target,
            initial_sequence: client.last_server_sequence(),
            initial: text_of(view.get(&target)),
            trace: Vec::new(),
            target_position: view.get(&target).map(|entity| entity.position),
            closing_step: None,
        })
    }

    fn record(&mut self, events: Vec<SessionEvent>) {
        for event in events {
            let SessionEvent::WorldSpatialEntities(delta) = event else {
                continue;
            };
            let value = delta.value;
            let line = if value.leave.contains(&self.target) {
                self.target_position = None;
                "absent".to_owned()
            } else if let Some(entity) = value
                .enter
                .iter()
                .chain(&value.update)
                .find(|entity| entity.entity == self.target)
            {
                let player = value.actor_position;
                let after = entity.position;
                if let Some(before) = self.target_position
                    && closes_distance(before, after, player)
                {
                    self.closing_step.get_or_insert((before, after, player));
                }
                self.target_position = Some(after);
                text_of(Some(entity))
            } else {
                "same".to_owned()
            };
            self.trace.push(format!("{}:{line}", delta.server_sequence));
        }
    }

    fn write_expectations(&self, expected: &mut String) {
        expected.push_str(&format!(
            "{n}.target={}:{}\n{n}.initial={}:{}\n{n}.trace={}\n",
            hex(&self.target.identity),
            self.target.generation,
            self.initial_sequence,
            self.initial,
            self.trace.join(";"),
            n = self.name,
        ));
    }
}

fn target_state(client: &DevClientSession, target: EntityRef) -> Option<WorldSpatialEntity> {
    client
        .world_entities()
        .and_then(|view| view.get(&target))
        .copied()
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
    let mut client = connect_world(clients, 0x0b).await?;
    let target = super::full_row_w01a::approach_rat(&mut client, world, &metadata.spawns).await?;
    let here = client.world_spatial().actor_position;

    // Chase: retreat with ordinary steps, then record the rat closing the distance.
    let target_tile = tile(target.position)?;
    let retreat = route(world, tile(here)?, |at| {
        at.floor == target_tile.floor
            && at
                .x
                .abs_diff(target_tile.x)
                .max(at.y.abs_diff(target_tile.y))
                >= 3
    })
    .ok_or("W01a client capture: no retreat lane")?;
    client.take_events();
    let mut chase = Segment::begin(&client, "chase", target.entity)?;
    let start = target_state(&client, target.entity)
        .map(|entity| entity.position)
        .ok_or("W01a client capture: rat vanished before chase")?;
    for direction in retreat {
        super::full_row_w01a::planned_step(&mut client, world, direction).await?;
        chase.record(client.take_events());
    }
    let player = client.world_spatial().actor_position;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let chased = loop {
        if let Some((before, after, observed_player)) = chase.closing_step {
            let current = target_state(&client, target.entity)
                .ok_or("W01a client capture: rat vanished after chase")?;
            evidence(&format!(
                "W01A_CLIENT_CHECK chase PASS from={before:?} to={after:?} player={observed_player:?}"
            ));
            break current.position;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "W01a client capture: rat never closed the distance from={start:?} player={player:?} live={:?}",
                target_state(&client, target.entity).map(|entity| entity.position)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        chase.record(client.take_events());
    };

    // Damage and flee: attack until the low-health boundary, stop, and record the retreat.
    let chased_tile = tile(chased)?;
    let approach = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == chased_tile.floor
            && at
                .x
                .abs_diff(chased_tile.x)
                .max(at.y.abs_diff(chased_tile.y))
                <= 1
    })
    .ok_or("W01a client capture: rat unreachable")?;
    super::full_row_w01a::walk_once(&mut client, world, &approach).await?;
    let modes = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        secure: true,
    };
    let mode_reply = client.session_mut().set_fight_modes(&modes).await?;
    if mode_reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01a client capture: fight modes refused: {mode_reply:?}").into());
    }
    client.take_events();
    let mut fight = Segment::begin(&client, "fight", target.entity)?;
    if !matches!(
        target_state(&client, target.entity).map(|entity| entity.detail),
        Some(
            EntityDetail::Actor {
                health_percent: 26..=100,
                ..
            } | EntityDetail::ActorWithIcons {
                health_percent: 26..=100,
                ..
            }
        )
    ) {
        return Err("W01a client capture: rat was already at or below the flee boundary".into());
    }
    let player_before_attack = client.world_spatial().actor_position;
    let mana_before_attack = client.actor_vitals().map(|vitals| vitals.mana);
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01a client capture: attack refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let (low, low_position) = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        fight.record(client.take_events());
        let Some(entity) = target_state(&client, target.entity) else {
            return Err("W01a client capture: rat died before the flee window".into());
        };
        if let EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } = entity.detail
            && health_percent <= 25
        {
            break (health_percent, entity.position);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "W01a client capture: rat never reached the low-health boundary initial={:?} rat={:?} player_before={player_before_attack:?} player_after={:?} mana_before={mana_before_attack:?} mana_after={:?}",
                fight.initial,
                (entity.position, entity.detail),
                client.world_spatial().actor_position,
                client.actor_vitals().map(|vitals| vitals.mana)
            )
            .into());
        }
    };
    let stopped = client.session_mut().attack_target(None).await?;
    if stopped.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01a client capture: stop attack refused: {stopped:?}").into());
    }
    let player = client.world_spatial().actor_position;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let fled = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        fight.record(client.take_events());
        if let Some(entity) = target_state(&client, target.entity)
            && entity.position != low_position
            && distance(entity.position, player) > distance(low_position, player)
        {
            break entity.position;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W01a client capture: rat never fled".into());
        }
    };

    let mut expected = format!(
        "character={}\nsession={}\nlast_sequence={}\n",
        hex(&clients.characters[1]),
        hex(client.join_snapshot().game_session_id.as_bytes()),
        client.last_server_sequence(),
    );
    chase.write_expectations(&mut expected);
    fight.write_expectations(&mut expected);
    expected.push_str(&format!(
        "chase.final={},{},{}\nlow={low}\nflee.final={},{},{}\n",
        chased.x, chased.y, chased.floor, fled.x, fled.y, fled.floor
    ));
    Ok(expected)
}

async fn capture_journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let directory = required("E2E_ALL_CAPTURE_DIR")?;
    let directory = Path::new(&directory);
    let (expected, wire) =
        oteryn_dev_client::native_session::capture_inbound(journey(clients)).await;
    let expected = expected?;
    if wire.is_empty() {
        return Err("W01a inbound capture is empty".into());
    }
    capture(directory, "w01a-inbound.bin", &wire)?;
    capture(directory, "w01a-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "{:x}:{:x}",
        Sha256::digest(&wire),
        Sha256::digest(expected.as_bytes())
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(capture_journey(clients)),
    )
    .await
    {
        Ok(Ok(digests)) => evidence(&format!(
            "e2e_all scenario=W01a.client status=CAPTURED evidence={digests}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W01a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W01a.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chase_uses_the_player_position_in_the_movement_delta() {
        let before = ActorPosition {
            x: 32387,
            y: 32252,
            floor: 8,
        };
        let after = ActorPosition { x: 32388, ..before };
        let observed_player = ActorPosition {
            x: 32389,
            y: 32251,
            ..before
        };
        let end_of_retreat = ActorPosition { y: 32255, ..after };
        assert!(closes_distance(before, after, observed_player));
        assert!(!closes_distance(before, after, end_of_retreat));
        assert!(!closes_distance(after, before, observed_player));
        assert!(!closes_distance(
            before,
            after,
            ActorPosition {
                floor: 7,
                ..observed_player
            }
        ));
    }
}
