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

const WORDS: &str = "exani tera";
const MANA: u32 = 20;
const ROPE_GROUNDS: &[u32] = &[
    386, 421, 7762, 12202, 12936, 14238, 17238, 23363, 21965, 21966, 21967, 21968,
];
const ROPE_TOPS: &[u32] = &[12935];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(180), journey(clients)).await;
    let (status, evidence_text) = match result {
        Ok(Ok(())) => ("PASS", None),
        Ok(Err(error)) => {
            let detail = error.to_string();
            evidence(&format!("M11B_GAP observed={detail}"));
            ("FAIL", None::<&str>)
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
            let rope = tile.items.iter().any(|item| {
                crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                    .is_some_and(|id| {
                        let id = u32::from(id);
                        let ground = manifest.palette[item.palette as usize].terrain.is_some_and(
                            |terrain| {
                                terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                            },
                        );
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
    let path = route(world, start, |at| spots.binary_search(&at).is_ok())
        .ok_or("step-1:no reachable compiled-world rope spot within player route budget")?;
    walk(&mut client, world, &path).await?;
    let rope = tile(client.world_spatial().actor_position)?;
    if spots.binary_search(&rope).is_err() {
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
    fn rope_catalogue_ids_are_disjoint() {
        assert!(ROPE_GROUNDS.iter().all(|id| !ROPE_TOPS.contains(id)));
    }
}
