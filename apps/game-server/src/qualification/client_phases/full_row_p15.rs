//! P15 concrete player scenario, derived from the matrix/audit and the exercise owner:
//! 1. Admit the provisioned character and send STATUS; expect an accepted, stopped result.
//! 2. Send START with non-existent player-visible handles; expect the owner's refused result and
//!    no charge/revision delta (the negative path).
//! 3. Buy an exercise weapon with player-earned gold when needed, and walk to an authored
//!    exercise dummy. Verify out-of-range refusal before starting within range.
//! 4. Wait through a two-second exercise tick, send STATUS, and expect exactly one or more charges
//!    spent while the durable character revision remains valid.
//! 5. Send STOP and expect stopped/settled state, the reduced charge count, and a nondecreasing
//!    durable revision.
//! 6. Logout, freshly admit the same character, reacquire the new-session item handles and send
//!    START then STOP; expect the persisted reduced charge count rather than the full definition
//!    count, proving both durable charge recovery and that the old exercise run did not resume.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{item_view::ItemHandle, loop_training as wire};

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
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "server command status={:?}; need=accepted exercise reply",
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

fn carried_weapon(client: &DevClientSession) -> TestResult<(ItemHandle, u32, u16)> {
    const WEAPONS: &[(u32, u32)] = &[
        (28540, 50),
        (28541, 50),
        (28542, 50),
        (28543, 50),
        (28544, 50),
        (28545, 50),
        (28552, 500),
        (28553, 500),
        (28554, 500),
        (28555, 500),
        (28556, 500),
        (28557, 500),
        (44064, 50),
        (44065, 500),
        (50292, 50),
        (50293, 500),
    ];
    let index = provisioning::item_index()?;
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    for &(id, charges) in WEAPONS {
        let reference = provisioning::item_reference(&index, id)?;
        if let Some(item) = inventory
            .entries
            .iter()
            .chain(&inventory.main_backpack)
            .find(|item| item.item_definition_ref == reference)
        {
            return Ok((item.handle, charges, weapon_range(id)));
        }
    }
    Err("no legitimately acquired exercise weapon in inventory after real refusal; need=reachable affordable NPC/store exercise-weapon offer".into())
}

fn weapon_range(id: u32) -> u16 {
    if matches!(id, 28543..=28545 | 28555..=28557) {
        7
    } else {
        1
    }
}

fn dummy_distance(at: TilePos, dummy: TilePos) -> Option<u16> {
    (at.floor == dummy.floor).then(|| at.x.abs_diff(dummy.x).max(at.y.abs_diff(dummy.y)))
}

fn visible_dummy(
    client: &mut DevClientSession,
    select: impl Fn(TilePos) -> bool,
) -> TestResult<(ItemHandle, TilePos)> {
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};

    let index = provisioning::item_index()?;
    let refs: std::collections::BTreeSet<_> = (28558..=28565)
        .map(|id| provisioning::item_reference(&index, id))
        .collect::<TestResult<_>>()?;
    let map = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("world-map snapshot missing")?;
    for map_tile in &map.tiles {
        let at = tile(map_tile.position)?;
        if !select(at) {
            continue;
        }
        for item in &map_tile.items {
            if let (MapDefinition::Item(reference), MapOrigin::Handle(handle)) =
                (item.definition, item.origin)
                && refs.contains(&reference)
            {
                return Ok((handle, at));
            }
        }
    }
    Err("step=exercise-dummy need=visible-authored-dummy-handle-at-planned-position".into())
}

fn dummy_positions(bytes: &[u8]) -> TestResult<std::collections::BTreeSet<TilePos>> {
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

async fn journey(verdict: &mut Verdict, clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_character(clients, 0xb5, 0).await?;
    let initial = command(&mut client, wire::ExerciseIntent::Status).await?;
    if initial.disposition != 2 || initial.revision == 0 {
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
    )
    .await?;
    if refused != wire::TrainingResult::REFUSED {
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
    if carried_weapon(&client).is_err() {
        let definitions = provisioning::item_index()?;
        super::full_e2ef7::acquire(
            &mut client,
            world,
            &definitions,
            "oteryn:item.tibia.i28552",
            1,
        )
        .await
        .map_err(|error| format!("step=exercise-weapon-acquisition item=i28552 observed={error}; source:content/services/trade/trade-00000-00323.json:1"))?;
    }
    let (weapon, full, range) = carried_weapon(&client)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        positions
            .iter()
            .any(|dummy| dummy_distance(at, *dummy) == Some(1))
    })
    .ok_or("no player-walkable route to authored exercise dummy; need=reachable dummy placement")?;
    walk(&mut client, world, &path).await?;
    let nearby = tile(client.world_spatial().actor_position)?;
    let (_, target) = visible_dummy(&mut client, |at| dummy_distance(nearby, at) == Some(1))?;
    let outside = route(world, nearby, |at| dummy_distance(at, target) == Some(range + 1))
        .ok_or("step=exercise-dummy-range need=reachable-tile-just-outside-weapon-range; source:crates/gameplay/dispatch/src/gameplay_transport/loop_training/exercise.rs:454")?;
    walk(&mut client, world, &outside).await?;
    let (dummy, _) = visible_dummy(&mut client, |at| at == target)?;
    let stopped_before_range_probe = command(&mut client, wire::ExerciseIntent::Status).await?;
    let refused = command(&mut client, wire::ExerciseIntent::Start { weapon, dummy }).await?;
    if refused != wire::TrainingResult::REFUSED
        || command(&mut client, wire::ExerciseIntent::Status).await? != stopped_before_range_probe
    {
        return Err(format!("step=exercise-dummy-range distance={} limit={range} observed={refused:?} need=inert-refusal", range + 1).into());
    }
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == nearby
    })
    .ok_or("step=exercise-dummy-range no route back to the legal use tile")?;
    walk(&mut client, world, &back).await?;
    let (dummy, _) = visible_dummy(&mut client, |at| at == target)?;
    let running = command(&mut client, wire::ExerciseIntent::Start { weapon, dummy }).await?;
    if running.disposition != 1 || running.charges != full || running.revision == 0 {
        return Err(format!("START reply={running:?}; need=running charges={full}").into());
    }
    verdict.pass(3, running);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    let ticked = loop {
        client.service_liveness(Duration::from_millis(200)).await?;
        let status = command(&mut client, wire::ExerciseIntent::Status).await?;
        if status.disposition != 1
            || status.charges < full
            || tokio::time::Instant::now() >= deadline
        {
            break status;
        }
    };
    if ticked.disposition != 1 || ticked.charges >= full {
        return Err(
            format!("post-tick STATUS reply={ticked:?}; need=charge delta below {full}").into(),
        );
    }
    verdict.pass(4, ticked);

    let stopped = command(&mut client, wire::ExerciseIntent::Stop).await?;
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
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout reply refused; need=accepted durable save".into());
    }
    drop(client);
    let mut client = connect_character(clients, 0xb6, 0).await?;
    let (weapon, _, _) = carried_weapon(&client)?;
    let (dummy, _) = visible_dummy(&mut client, |at| at == target)?;
    let restored = command(&mut client, wire::ExerciseIntent::Start { weapon, dummy }).await?;
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
    let restored = command(&mut client, wire::ExerciseIntent::Stop).await?;
    if restored.disposition != 2 || restored.charges != stopped.charges {
        return Err(
            format!("fresh-admission STOP={restored:?}; need=settled persisted charges").into(),
        );
    }
    verdict.pass(6, restored);
    let _ = client.session_mut().logout().await?;
    Ok(())
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
