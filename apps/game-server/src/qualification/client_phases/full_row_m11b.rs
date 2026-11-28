//! M11b — Magic Rope (`exani tera`) full-row player scenario.
//!
//! 1. Admit the node-boot character and walk, using native STEP commands, from
//!    its persisted position onto a reachable compiled-world rope spot.
//! 2. Say `exani tera`; require an accepted chat reply, the spell's 20-mana
//!    debit, and a pushed position delta to an adjacent tile one floor above.
//! 3. Walk off the rope spot, say `exani tera` again after cooldown, and require
//!    the refusal path to leave both position and mana unchanged.
//! 4. Log out, obtain a fresh admission, and require a new game-session id and
//!    the final refusal tile to remain the durable admitted position.
//!
//! The server owner is `crates/gameplay/spell/src/spell/world_execution.rs`.
//! A reached rope cast that is accepted but denied solely by the bundle landing
//! flags is reported at run time as MISSING-SERVER, with that owner path.
use super::*;
use oteryn_dev_client::{
    ChatDisposition, ChatIntent, ChatOutcome, ChatSpeechMode, DevClientSession,
};

pub(super) const APPROACH: TilePos = TilePos {
    x: 32296,
    y: 32331,
    floor: -7,
};
pub(super) const ROPE: TilePos = TilePos {
    x: 32296,
    y: 32334,
    floor: -8,
};

pub(super) fn isolated_batch() -> bool {
    std::env::var("NODE_BOOT_E2E_ONLY").as_deref() == Ok("rows")
        && matches!(
            std::env::var("OTERYN_E2E_ROW_BATCH").as_deref(),
            Ok("70:1" | "139:1")
        )
}

const WORDS: &str = "exani tera";
const MANA: u32 = 20;
const ROPE_GROUNDS: &[u32] = &[
    386, 421, 7762, 12202, 12936, 14238, 17238, 23363, 21965, 21966, 21967, 21968,
];
const ROPE_TOPS: &[u32] = &[12935];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let (status, evidence_text) = match result {
        Ok(Ok(())) => ("PASS", None),
        Ok(Err(error)) => {
            let detail = error.to_string();
            evidence(&format!("M11B_GAP observed={detail}"));
            if detail.starts_with("landing-denied:") {
                (
                    "MISSING-SERVER",
                    Some("source:crates/gameplay/spell/src/spell/world_execution.rs:516"),
                )
            } else {
                ("FAIL", None)
            }
        }
        Err(_) => {
            evidence("M11B_GAP observed=journey-timeout");
            ("FAIL", None)
        }
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=M11b.full status={status} seconds={:.3}{}",
        started.elapsed().as_secs_f64(),
        evidence_text.map_or(String::new(), |value| format!(" evidence={value}")),
    ));
}

async fn say(client: &mut DevClientSession) -> TestResult<ChatOutcome> {
    Ok(client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: WORDS.into(),
        })
        .await?)
}

fn rope_spots(bytes: &[u8]) -> TestResult<Vec<TilePos>> {
    let mut spots = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            let rope = tile
                .items
                .iter()
                .filter(|item| item.depth == 0)
                .any(|item| {
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                        .is_some_and(|id| {
                            let id = u32::from(id);
                            let ground = manifest.palette[item.palette as usize]
                                .terrain
                                .is_some_and(|terrain| {
                                    terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                                });
                            (ground && ROPE_GROUNDS.contains(&id))
                                || (!ground && ROPE_TOPS.contains(&id))
                        })
                });
            if rope {
                spots.push(TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    spots.sort();
    spots.dedup();
    Ok(spots)
}

async fn settle(client: &mut DevClientSession) -> TestResult {
    let until = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

fn has_static_landing(rope: TilePos, walkable: impl Fn(TilePos) -> bool) -> bool {
    let Some(floor) = rope.floor.checked_add(1).filter(|floor| *floor <= 0) else {
        return false;
    };
    (-1..=1).any(|dx| {
        (-1..=1).any(|dy| {
            if (dx, dy) == (0, 0) {
                return false;
            }
            let (Some(x), Some(y)) = (rope.x.checked_add_signed(dx), rope.y.checked_add_signed(dy))
            else {
                return false;
            };
            walkable(TilePos { x, y, floor })
        })
    })
}

// Follow the selected route exactly. The committed server wait schedules the next
// step while the client continues servicing pushed state and liveness frames.
pub(super) async fn walk_to_rope(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    use oteryn_dev_client::EntityKind;

    evidence(&format!("M11B_ROUTE steps={}", path.len()));
    for (index, &direction) in path.iter().enumerate() {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("M11b route has no landing")?;
        evidence(&format!(
            "M11B_STEP_SEND index={index} from={from:?} expected={expected:?} direction={direction:?} server_sequence={}",
            client.last_server_sequence(),
        ));
        let mut outcome = client.step(direction).await.inspect_err(|error| {
            evidence(&format!(
                "M11B_STEP_UNACKNOWLEDGED index={index} server_sequence={} error={error:?}",
                client.last_server_sequence()
            ));
        })?;
        evidence(&format!(
            "M11B_STEP_REPLY index={index} command={} server_sequence={} disposition={:?} wait_ms={}",
            outcome.command_id,
            outcome.result_server_sequence,
            outcome.disposition,
            outcome.step_wait_ms,
        ));
        if outcome.disposition == StepDisposition::Blocked {
            if tile(client.world_spatial().actor_position)? != from {
                return Err("M11b blocked step moved the actor".into());
            }
            let blocker = client.world_entities().and_then(|entities| {
                entities.iter().find(|actor| {
                    matches!(actor.kind, EntityKind::Creature | EntityKind::Npc | EntityKind::Player)
                        && tile(actor.position).ok() == Some(expected)
                })
            }).map(|actor| actor.entity).ok_or_else(|| {
                format!("BLOCKER-SERVER M11b blocked with no occupant at {expected:?} command={} server_sequence={}",
                    outcome.command_id, outcome.result_server_sequence)
            })?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
            loop {
                client.service_liveness(Duration::from_millis(50)).await?;
                let entities = client.world_entities().ok_or("M11b occupant view lost")?;
                let position = entities
                    .iter()
                    .find(|actor| actor.entity == blocker)
                    .map(|actor| tile(actor.position))
                    .transpose()?;
                if position != Some(expected) {
                    if entities.iter().any(|actor| {
                        matches!(
                            actor.kind,
                            EntityKind::Creature | EntityKind::Npc | EntityKind::Player
                        ) && tile(actor.position).ok() == Some(expected)
                    }) {
                        return Err(format!(
                            "M11b landing acquired a new occupant at {expected:?}"
                        )
                        .into());
                    }
                    evidence(&format!(
                        "M11B_STEP_CLEAR index={index} blocker={blocker:?} position={position:?} server_sequence={}",
                        client.last_server_sequence()
                    ));
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("M11b occupant {blocker:?} stayed at {expected:?}").into());
                }
            }
            outcome = client.step(direction).await.inspect_err(|error| {
                evidence(&format!("M11B_STEP_UNACKNOWLEDGED index={index} after_clear=true server_sequence={} error={error:?}", client.last_server_sequence()));
            })?;
            evidence(&format!(
                "M11B_STEP_AFTER_CLEAR index={index} command={} server_sequence={} disposition={:?}",
                outcome.command_id, outcome.result_server_sequence, outcome.disposition
            ));
        }
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "M11b step {index} failed from={from:?} expected={expected:?}: {outcome:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        if index + 1 < path.len() && outcome.step_wait_ms > 0 {
            let until = tokio::time::Instant::now()
                + Duration::from_millis(u64::from(outcome.step_wait_ms));
            while let Some(remaining) = until.checked_duration_since(tokio::time::Instant::now()) {
                if remaining.is_zero() {
                    break;
                }
                client
                    .service_liveness(remaining.min(Duration::from_millis(50)))
                    .await?;
            }
        }
        client.take_events();
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let spots = rope_spots(&bytes)?;
    let mut client = connect_character(clients, 0xb1, 1).await?;
    let start = tile(client.world_spatial().actor_position)?;
    if isolated_batch() && start != APPROACH {
        return Err(format!("M11b fixture entered at {start:?}, expected {:?}", APPROACH).into());
    }
    let path = route(world, start, |at| {
        at == ROPE && spots.binary_search(&at).is_ok()
            && has_static_landing(at, |landing| {
                world.base().tile(landing.x, landing.y, landing.floor)
                    .is_some_and(|tile| tile.walkable() && world.enterable(landing))
            })
    }).ok_or("step=rope-route missing=reachable-rope-spot-with-walkable-upstairs-neighbour owner=crates/gameplay/spell/src/spell/world_execution.rs:482")?;
    walk_to_rope(&mut client, world, &path).await?;
    let rope = tile(client.world_spatial().actor_position)?;
    if rope != ROPE || start == ROPE {
        return Err("step-1:STEP replies did not finish on rope spot".into());
    }
    evidence(&format!("M11B_CHECK step=1 status=PASS rope={rope:?}"));

    let before_mana = client
        .actor_vitals()
        .ok_or("step-2:missing pre-cast vitals")?
        .mana;
    let cast = say(&mut client).await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!(
            "step-2:cast reply status={:?} disposition={:?}",
            cast.status, cast.disposition
        )
        .into());
    }
    settle(&mut client).await?;
    let landed = tile(client.world_spatial().actor_position)?;
    let after_mana = client
        .actor_vitals()
        .ok_or("step-2:missing post-cast vitals")?
        .mana;
    if landed == rope {
        return Err(format!("landing-denied:step-2 accepted reply status={:?} disposition={:?}, position unchanged={landed:?}, mana={before_mana}->{after_mana}", cast.status, cast.disposition).into());
    }
    if landed.floor != rope.floor + 1
        || landed.x.abs_diff(rope.x).max(landed.y.abs_diff(rope.y)) != 1
        || before_mana.checked_sub(MANA) != Some(after_mana)
    {
        return Err(format!("step-2:bad landing/delta rope={rope:?} landed={landed:?} mana={before_mana}->{after_mana}").into());
    }
    evidence(&format!("M11B_CHECK step=2 status=PASS landing={landed:?}"));

    let away = route(world, landed, |at| {
        at.floor == landed.floor && !spots.contains(&at) && at != landed
    })
    .ok_or("step-3:no walkable non-rope refusal tile")?;
    walk(&mut client, world, &away).await?;
    settle(&mut client).await?;
    let refused_at = tile(client.world_spatial().actor_position)?;
    let refused_mana = client
        .actor_vitals()
        .ok_or("step-3:missing refusal vitals")?
        .mana;
    let refused = say(&mut client).await?;
    settle(&mut client).await?;
    if refused.status != CommandStatus::Accepted
        || tile(client.world_spatial().actor_position)? != refused_at
        || client.actor_vitals().is_none_or(|v| v.mana != refused_mana)
    {
        return Err(format!("step-3:refusal reply={refused:?} position/mana changed").into());
    }
    evidence("M11B_CHECK step=3 status=PASS unchanged-position-and-mana");

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-4:logout refused".into());
    }
    let old_session = client.join_snapshot().game_session_id;
    drop(client);
    let mut fresh = connect_character(clients, 0xb2, 1).await?;
    if fresh.join_snapshot().game_session_id == old_session
        || tile(fresh.world_spatial().actor_position)? != refused_at
    {
        return Err("step-4:fresh admission lost durable final position or reused session".into());
    }
    evidence("M11B_CHECK step=4 status=PASS fresh-admission-retained-position");
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-4:fresh cleanup logout refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rope_planner_requires_an_upstairs_neighbour() {
        let rope = TilePos {
            x: 5,
            y: 6,
            floor: -8,
        };
        assert!(!has_static_landing(rope, |at| at == TilePos { floor: -7, ..rope }));
        assert!(has_static_landing(rope, |at| at
            == TilePos {
                x: 6,
                y: 7,
                floor: -7
            }));
        assert!(!has_static_landing(TilePos { floor: 0, ..rope }, |_| true));
    }

    #[test]
    fn rope_catalogue_ids_are_disjoint() {
        assert!(ROPE_GROUNDS.iter().all(|id| !ROPE_TOPS.contains(id)));
    }
}
