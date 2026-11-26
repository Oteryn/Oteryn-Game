//! P15 concrete player scenario, derived from the matrix/audit and the exercise owner:
//! 1. Admit the provisioned character and send STATUS; expect an accepted, stopped result.
//! 2. Send START with non-existent player-visible handles; expect the owner's refused result and
//!    no charge/revision delta (the negative path).
//! 3. Walk to an authored exercise dummy with the fixture's exercise sword.
//!    Verify out-of-range refusal before starting within range.
//! 4. Wait through a two-second exercise tick, send STATUS, and expect exactly one or more charges
//!    spent while the durable character revision remains valid.
//! 5. Send STOP and expect stopped/settled state, the reduced charge count, and a nondecreasing
//!    durable revision.
//! 6. Logout, freshly admit the same character, reacquire the new-session item handles and send
//!    START then STOP; expect the persisted reduced charge count rather than the full definition
//!    count, proving both durable charge recovery and that the old exercise run did not resume.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{
    item_view::ItemHandle,
    loop_training as wire,
    world_map::{MapDefinition, MapOrigin, MapTile},
};

const EXERCISE_SWORD: u32 = 28552;

struct Verdict {
    started: std::time::Instant,
    step: usize,
    failure: Option<String>,
}

impl Verdict {
    fn pass(&mut self, step: usize, observed: impl std::fmt::Debug) {
        self.step = step;
        evidence(&format!(
            "P15_CHECK step={step} status=PASS observed={observed:?}"
        ));
    }
    fn finish(self) {
        if let Some(failure) = &self.failure {
            evidence(&format!(
                "P15_GAP step={} observed={failure}",
                self.step + 1
            ));
        }
        evidence(&format!(
            "e2e_all scenario=P15.full status={} seconds={:.3}{}",
            if self.failure.is_none() && self.step == 6 {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
            self.failure
                .as_ref()
                .map_or(String::new(), |failure| format!(
                    " evidence=step-{}:{failure}",
                    self.step + 1
                )
                .replace(' ', "_")),
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut verdict = Verdict {
        started: std::time::Instant::now(),
        step: 0,
        failure: None,
    };
    let result = verdict::indexed_timeout(
        Duration::from_secs(300),
        verdict::observe(journey(&mut verdict, clients)),
    )
    .await;
    if let Err(error) = match result {
        Ok(result) => result,
        Err(_) => Err("step deadline; observed=timeout; need=responsive exercise owner".into()),
    } {
        verdict.failure = Some(error.to_string());
    }
    verdict.finish();
}

async fn command(
    client: &mut DevClientSession,
    intent: wire::ExerciseIntent,
    expected_status: CommandStatus,
) -> TestResult<wire::TrainingResult> {
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
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TRAINING_V1,
            wire::COMMAND_TYPE_EXERCISE_TRAINING,
            &payload,
        )
        .await?;
    if reply.status != expected_status {
        return Err(format!(
            "exercise {intent:?} command status={:?}; expected={expected_status:?}",
            reply.status
        )
        .into());
    }
    decode_result(&reply.payload).ok_or_else(|| {
        format!(
            "malformed training reply={:?}; need=TrainingResult",
            reply.payload
        )
        .into()
    })
}

fn push_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
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

fn decode_result(bytes: &[u8]) -> Option<wire::TrainingResult> {
    if bytes.len() > wire::MAX_TRAINING_RESULT_BYTES {
        return None;
    }
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
    wire::encode_training_result(result).ok()?;
    Some(result)
}

pub(super) fn carried_weapon(client: &DevClientSession) -> TestResult<(ItemHandle, u32, u16)> {
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, EXERCISE_SWORD)?;
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    let item = inventory
        .entries
        .iter()
        .find(|item| item.item_definition_ref == reference)
        .ok_or("fixture exercise sword missing from the main backpack")?;
    Ok((item.handle, 500, weapon_range(EXERCISE_SWORD)))
}

fn weapon_range(id: u32) -> u16 {
    if matches!(id, 28543..=28545 | 28555..=28557) {
        7
    } else {
        1
    }
}

pub(super) fn dummy_distance(at: TilePos, dummy: TilePos) -> Option<u16> {
    (at.floor == dummy.floor).then(|| at.x.abs_diff(dummy.x).max(at.y.abs_diff(dummy.y)))
}

pub(super) struct DummyView {
    target: ActorPosition,
    origin: ActorPosition,
    tile: Option<MapTile>,
}

impl DummyView {
    pub(super) fn admitted(client: &mut DevClientSession, target: TilePos) -> TestResult<Self> {
        let target = ActorPosition {
            x: i32::from(target.x),
            y: i32::from(target.y),
            floor: -i16::from(target.floor),
        };
        let map = client
            .session_mut()
            .world_map_snapshot()
            .ok_or("world-map admission snapshot missing")?;
        Ok(Self {
            target,
            origin: map.header.origin,
            tile: map.tiles.into_iter().find(|tile| tile.position == target),
        })
    }

    fn observe(&mut self, client: &mut DevClientSession) {
        for event in client.take_events() {
            match event {
                SessionEvent::WorldMap(delta) => {
                    self.origin = delta.value.header.origin;
                    if delta.value.cleared.contains(&self.target) {
                        self.tile = None;
                    }
                    if let Some(tile) = delta
                        .value
                        .tiles
                        .into_iter()
                        .find(|tile| tile.position == self.target)
                    {
                        self.tile = Some(tile);
                    }
                }
                SessionEvent::WorldMapSnapshot(snapshot) => {
                    self.origin = snapshot.value.header.origin;
                    self.tile = snapshot
                        .value
                        .tiles
                        .into_iter()
                        .find(|tile| tile.position == self.target);
                }
                _ => {}
            }
        }
    }

    pub(super) async fn handle(&mut self, client: &mut DevClientSession) -> TestResult<ItemHandle> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            self.observe(client);
            if self.origin == client.world_spatial().actor_position {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "current world-map origin missing for planned dummy {:?}",
                    self.target
                )
                .into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
        let index = provisioning::item_index()?;
        let refs = (28558..=28565)
            .map(|id| provisioning::item_reference(&index, id))
            .collect::<TestResult<Vec<_>>>()?;
        let item = self.tile.as_ref().and_then(|tile| {
            tile.items.iter().find(|item| {
                matches!(item.definition, MapDefinition::Item(reference) if refs.contains(&reference))
            })
        }).ok_or_else(|| format!("live map missing authored exercise dummy at {:?}", self.target))?;
        match item.origin {
            MapOrigin::Handle(handle) => Ok(handle),
            origin => Err(format!("exercise dummy at {:?} has live map origin {origin:?}; need a handle for EXERCISE_TRAINING", self.target).into()),
        }
    }
}

pub(super) async fn walk_to_probe(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    dummy: &mut DummyView,
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("planned STEP has no landing")?;
        let outcome = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!("exercise STEP {direction:?} from={from:?} expected={expected:?} actual={actual:?}: {outcome:?}").into());
        }
        verdict::Harness::global().progress("walk");
        dummy.observe(client);
    }
    Ok(())
}

pub(super) fn dummy_positions(bytes: &[u8]) -> TestResult<std::collections::BTreeSet<TilePos>> {
    let mut positions = std::collections::BTreeSet::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            if tile.items.iter().any(|item| {
                let key = &manifest.palette[item.palette as usize].key;
                (28558..=28565).any(|id| key == &format!("oteryn:item.tibia.i{id}"))
            }) {
                positions.insert(TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    Ok(positions)
}

pub(super) async fn exercise_character(
    clients: &SeamClients<'_>,
    row: &str,
    tag: u8,
) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        row,
        "exercise sword i28552",
        "exercise range and charge persistence",
    );
    let setup = connect_character(clients, 0xb4, 0).await?;
    character_prep::grant_item(&setup, EXERCISE_SWORD, 1).await?;
    let setup_session = setup.join_snapshot().game_session_id;
    // The fixture writes outside the player's inventory stream. End its
    // transport and let fresh admission wait for the normal terminal release.
    drop(setup);

    let client = connect_character(clients, tag, 0).await?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("exercise journey reused the fixture session".into());
    }
    Ok(client)
}

async fn journey(verdict: &mut Verdict, clients: &SeamClients<'_>) -> TestResult {
    use CommandStatus::{Accepted, Rejected};

    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_P15_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let mut client = exercise_character(clients, "P15.full", 0xb5).await?;
    let initial = command(&mut client, wire::ExerciseIntent::Status, Accepted).await?;
    if initial.disposition != 2 || initial.revision == 0 || initial.charges != 0 {
        return Err(format!("STATUS reply={initial:?}; need=stopped durable state").into());
    }
    verdict.pass(1, initial);

    let missing = ItemHandle::new(u64::MAX).ok_or("nonzero refusal handle")?;
    let refused = command(
        &mut client,
        wire::ExerciseIntent::Start {
            weapon: missing,
            dummy: missing,
        },
        Rejected,
    )
    .await?;
    if refused != wire::TrainingResult::REFUSED
        || command(&mut client, wire::ExerciseIntent::Status, Accepted).await? != initial
    {
        return Err(format!("invalid START reply={refused:?}; need=REFUSED without delta").into());
    }
    verdict.pass(2, refused);

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let positions = dummy_positions(&bytes)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let (weapon, full, range) = carried_weapon(&client)?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        positions
            .iter()
            .any(|dummy| dummy_distance(at, *dummy) == Some(1))
    })
    .ok_or("no player-walkable route to authored exercise dummy; need=reachable dummy placement")?;
    let nearby = path.iter().try_fold(from, |at, &direction| {
        destination(world, at, direction).ok_or("planned exercise route has no landing")
    })?;
    let target = positions
        .into_iter()
        .find(|&target| dummy_distance(nearby, target) == Some(1))
        .ok_or("planned destination has no adjacent exercise dummy")?;
    let mut dummy_view = DummyView::admitted(&mut client, target)?;
    walk_to_probe(&mut client, world, &path, &mut dummy_view).await?;
    dummy_view.handle(&mut client).await?;
    let outside = route(world, nearby, |at| dummy_distance(at, target) == Some(range + 1))
        .ok_or("step=exercise-dummy-range need=reachable-tile-just-outside-weapon-range; source:crates/gameplay/dispatch/src/gameplay_transport/loop_training/exercise.rs:454")?;
    walk_to_probe(&mut client, world, &outside, &mut dummy_view).await?;
    let dummy = dummy_view.handle(&mut client).await?;
    let stopped_before_range_probe =
        command(&mut client, wire::ExerciseIntent::Status, Accepted).await?;
    let refused = command(
        &mut client,
        wire::ExerciseIntent::Start { weapon, dummy },
        Rejected,
    )
    .await?;
    if refused != wire::TrainingResult::REFUSED
        || command(&mut client, wire::ExerciseIntent::Status, Accepted).await?
            != stopped_before_range_probe
    {
        return Err(format!("step=exercise-dummy-range distance={} limit={range} observed={refused:?} need=inert-refusal", range + 1).into());
    }
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == nearby
    })
    .ok_or("step=exercise-dummy-range no route back to the legal use tile")?;
    walk_to_probe(&mut client, world, &back, &mut dummy_view).await?;
    let dummy = dummy_view.handle(&mut client).await?;
    let running = command(
        &mut client,
        wire::ExerciseIntent::Start { weapon, dummy },
        Accepted,
    )
    .await?;
    if running.disposition != 1 || running.charges != full || running.revision == 0 {
        return Err(format!("START reply={running:?}; need=running charges={full}").into());
    }
    verdict.pass(3, running);

    let ticked = observe_first_charge(&mut client, full).await?;
    verdict.pass(4, ticked);

    let stopped = command(&mut client, wire::ExerciseIntent::Stop, Accepted).await?;
    if stopped.disposition != 2
        || stopped.charges != ticked.charges
        || stopped.revision < ticked.revision
    {
        return Err(format!(
            "STOP reply={stopped:?}; before={ticked:?}; need=settled durable charge/revision"
        )
        .into());
    }
    verdict.pass(5, stopped);
    let restored = verify_charge_after_relog(client, clients, target, stopped, 0xb6).await?;
    verdict.pass(6, restored);
    Ok(())
}

pub(super) async fn observe_first_charge(
    client: &mut DevClientSession,
    full: u32,
) -> TestResult<wire::TrainingResult> {
    use CommandStatus::Accepted;

    let expected = full
        .checked_sub(1)
        .ok_or("exercise weapon has no charges")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    let ticked = loop {
        client.service_liveness(Duration::from_millis(200)).await?;
        let status = command(client, wire::ExerciseIntent::Status, Accepted).await?;
        if status.disposition != 1
            || status.charges != full
            || tokio::time::Instant::now() >= deadline
        {
            break status;
        }
    };
    if ticked.disposition != 1 || ticked.charges != expected {
        return Err(format!(
            "post-tick STATUS reply={ticked:?}; need=exact first-tick charge count {expected}"
        )
        .into());
    }
    Ok(ticked)
}

pub(super) async fn verify_charge_after_relog(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
    target: TilePos,
    stopped: wire::TrainingResult,
    tag: u8,
) -> TestResult<wire::TrainingResult> {
    use CommandStatus::Accepted;

    let session = client.join_snapshot().game_session_id;
    let before = client.world_spatial().actor_position;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout reply refused; need=accepted durable save".into());
    }
    drop(client);
    let mut client = connect_character(clients, tag, 0).await?;
    if client.join_snapshot().game_session_id == session
        || client.world_spatial().actor_position != before
    {
        return Err("fresh admission must restore the same exercise tile in a new session".into());
    }
    let relogged = command(&mut client, wire::ExerciseIntent::Status, Accepted).await?;
    if relogged.disposition != 2 || relogged.charges != 0 || relogged.revision < stopped.revision {
        return Err(format!("fresh-admission STATUS={relogged:?}; need stopped state").into());
    }
    let (weapon, _, _) = carried_weapon(&client)?;
    let dummy = DummyView::admitted(&mut client, target)?
        .handle(&mut client)
        .await?;
    let restored = command(
        &mut client,
        wire::ExerciseIntent::Start { weapon, dummy },
        Accepted,
    )
    .await?;
    if restored.disposition != 1
        || restored.charges != stopped.charges
        || restored.revision < stopped.revision
    {
        return Err(format!(
            "fresh-admission START={restored:?}; expected charges={} revision>={}",
            stopped.charges, stopped.revision
        )
        .into());
    }
    let restored = command(&mut client, wire::ExerciseIntent::Stop, Accepted).await?;
    if restored.disposition != 2 || restored.charges != stopped.charges {
        return Err(
            format!("fresh-admission STOP={restored:?}; need=settled persisted charges").into(),
        );
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("post-verification logout refused".into());
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_respects_floor_and_weapon_family() {
        let dummy = TilePos {
            x: 10,
            y: 20,
            floor: 7,
        };
        assert_eq!(
            dummy_distance(
                TilePos {
                    x: 12,
                    y: 21,
                    floor: 7
                },
                dummy
            ),
            Some(2)
        );
        assert_eq!(
            dummy_distance(
                TilePos {
                    x: 10,
                    y: 20,
                    floor: 6
                },
                dummy
            ),
            None
        );
        assert_eq!(weapon_range(28552), 1);
        assert_eq!(weapon_range(28557), 7);
    }

    #[test]
    fn training_reply_decoder_rejects_missing_fields() {
        assert_eq!(super::decode_result(&[8, 2]), None);
        let decoded = super::wire::encode_training_result(super::wire::TrainingResult {
            disposition: 2,
            revision: 7,
            pool_seconds: 0,
            charges: 499,
        })
        .ok()
        .and_then(|encoded| super::decode_result(&encoded));
        assert_eq!(decoded.map(|reply| reply.charges), Some(499));
    }
}
