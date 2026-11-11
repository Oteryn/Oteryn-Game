//! P15 concrete player scenario, derived from the matrix/audit and the exercise owner:
//! 1. Admit the provisioned character and send STATUS; expect an accepted, stopped result.
//! 2. Send START with non-existent player-visible handles; expect the owner's refused result and
//!    no charge/revision delta (the negative path).
//! 3. Locate an exercise weapon acquired by ordinary earlier player play and walk to an authored
//!    exercise dummy; send START and expect running state plus the weapon's full charge count.
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
                "P15_CHECK step={} status=FAIL observed={failure}",
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
    let result = row_complete(journey(&mut verdict, clients)).await;
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

fn carried_weapon(client: &DevClientSession) -> TestResult<(ItemHandle, u32)> {
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
            return Ok((item.handle, charges));
        }
    }
    Err("no legitimately acquired exercise weapon in inventory after real refusal; need=reachable affordable NPC/store exercise-weapon offer".into())
}

fn visible_dummy(client: &mut DevClientSession) -> TestResult<ItemHandle> {
    let index = provisioning::item_index()?;
    let refs: std::collections::BTreeSet<_> = (28558..=28565)
        .map(|id| provisioning::item_reference(&index, id))
        .collect::<TestResult<_>>()?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("world-map snapshot missing")?;
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("world-map snapshot: {error:?}"))?;
    map.tiles.iter().flat_map(|tile| &tile.items).find_map(|item| {
        match (item.definition, item.origin) {
            (oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference),
             oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle)) if refs.contains(&reference) => Some(handle),
            _ => None,
        }
    }).ok_or_else(|| "no authored exercise dummy in admitted map range after real refusal; need=reachable exercise-dummy placement".into())
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
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_character(clients, 0xb5, 0).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
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
            return Err(
                format!("invalid START reply={refused:?}; need=REFUSED without delta").into(),
            );
        }
        verdict.pass(2, refused);

        let (weapon, full) = carried_weapon(&client)?;
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let positions = dummy_positions(&bytes)?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let world = load_world(
            bytes,
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            positions.iter().any(|dummy| {
                at.floor == dummy.floor && at.x.abs_diff(dummy.x).max(at.y.abs_diff(dummy.y)) <= 1
            })
        })
        .ok_or(
            "no player-walkable route to authored exercise dummy; need=reachable dummy placement",
        )?;
        walk(&mut client, world, &path).await?;
        let dummy = visible_dummy(&mut client)?;
        let running = command(&mut client, wire::ExerciseIntent::Start { weapon, dummy }).await?;
        if running.disposition != 1 || running.charges != full || running.revision == 0 {
            return Err(format!("START reply={running:?}; need=running charges={full}").into());
        }
        verdict.pass(3, running);

        client.service_liveness(Duration::from_millis(2300)).await?;
        let ticked = command(&mut client, wire::ExerciseIntent::Status).await?;
        if ticked.disposition != 1 || ticked.charges >= full {
            return Err(format!(
                "post-tick STATUS reply={ticked:?}; need=charge delta below {full}"
            )
            .into());
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
        row_needs_logout_0 = false;
        if row_needs_logout_0 {
            row_logout(&mut client).await?;
            row_needs_logout_0 = false;
        }
        let mut client = connect_character(clients, 0xb6, 0).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            let (weapon, _) = carried_weapon(&client)?;
            let dummy = visible_dummy(&mut client)?;
            let restored =
                command(&mut client, wire::ExerciseIntent::Start { weapon, dummy }).await?;
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
                return Err(format!(
                    "fresh-admission STOP={restored:?}; need=settled persisted charges"
                )
                .into());
            }
            verdict.pass(6, restored);
            row_logout(&mut client).await?;
            row_needs_logout_1 = false;
            Ok(())
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            row_logout(&mut client).await?;
        }
        row_result
    }
    .await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

#[cfg(test)]
mod tests {
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
