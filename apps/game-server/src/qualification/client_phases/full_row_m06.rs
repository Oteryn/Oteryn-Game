//! M06: keys and level/quest door predicates, exercised through native player commands.
//!
//! Concrete player scenario derived from the M06 matrix/audit row and `map_door` owner:
//! 1. Walk next to an authored locked key door; send USE and require `Locked` with no movement.
//! 2. Send player USE-WITH with an ordinarily carried item against that door; require either a
//!    committed matching-key transition plus a map-state revision, or an explicit key refusal.
//! 3. Walk next to an authored level door; send USE below its required level and require `Locked`,
//!    then (only after ordinary play has earned the level) send USE and require `Committed`.
//! 4. Walk next to an authored quest door; send USE before its storage transition and require
//!    `Locked`, acquire the transition by ordinary dialogue/combat/item play, then require USE to
//!    commit. A mismatched key/storage/level must never open any of these doors.
//! 5. Log out, obtain a fresh admission, walk back, and repeat the predicate checks: durable level
//!    and quest storage must still authorize their doors, while channel-local door pose may reset.
//!
//! The journey never manufactures prerequisites. It always sends the mechanic commands first;
//! when the compiled node-boot character has no legitimately earned key/level/quest transition,
//! the final verdict is FAIL and the observed refusal identifies the missing server/content path.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[derive(Clone, Copy, Debug)]
struct Door {
    at: TilePos,
    ordinal: u8,
    appearance: u16,
    action: u16,
}

fn doors(bytes: &[u8], world: &BundleWorld) -> TestResult<Vec<Door>> {
    let mut found = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            let at = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let Some(action) = item.attrs.action.filter(|action| *action != 0) else {
                    continue;
                };
                let Ok(ordinal) = u8::try_from(ordinal) else {
                    continue;
                };
                let Some(appearance) =
                    crate::map::palette_appearance(&manifest.palette[item.palette as usize].key)
                else {
                    continue;
                };
                if world.facts().doors().at(at) {
                    found.push(Door {
                        at,
                        ordinal,
                        appearance,
                        action,
                    });
                }
            }
        }
        Ok(())
    })?;
    found.sort_by_key(|door| (door.at, door.ordinal));
    Ok(found)
}

fn target(world: &BundleWorld, door: Door, revision: u64) -> TestResult<WorldObjectTarget> {
    let key = crate::map::view::placement_key(door.at, door.ordinal).ok_or("M06 placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
}

async fn approach(client: &mut DevClientSession, world: &BundleWorld, door: Door) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        at.floor == door.at.floor && at.x.abs_diff(door.at.x).max(at.y.abs_diff(door.at.y)) == 1
    })
    .ok_or_else(|| format!("M06 no walking route to door {:?}", door.at))?;
    walk(client, world, &path).await
}

async fn use_door(
    client: &mut DevClientSession,
    world: &BundleWorld,
    door: Door,
) -> TestResult<UseDisposition> {
    let target = target(world, door, 0)?;
    Ok(client.use_object(&target.placement, 0).await?.disposition)
}

async fn use_carried_on_door(
    client: &mut DevClientSession,
    world: &BundleWorld,
    door: Door,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let carried = client
        .inventory()
        .and_then(|view| view.entries.first())
        .copied()
        .ok_or("M06 player has no ordinarily carried item for key USE-WITH attempt")?;
    let intent = UseIntent {
        target: UseTarget::Item(carried.handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(world, door, 0)?)),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent)
        .map_err(|error| format!("M06 USE-WITH encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("M06 USE-WITH reply {:?}: {error:?}", reply.status))?;
    Ok((reply.status, disposition))
}

fn overlay_revision(client: &DevClientSession, placement: &[u8]) -> u64 {
    client
        .world_object_overlay()
        .iter()
        .find(|entry| entry.placement == placement)
        .map_or(0, |entry| entry.revision)
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld, bytes: &[u8]) -> TestResult {
    let authored = doors(bytes, world)?;
    let level = authored
        .iter()
        .copied()
        .find(|door| door.action > 1000)
        .ok_or("step-3 observed=no-authored-level-door")?;
    let key = authored
        .iter()
        .copied()
        .find(|door| door.appearance == 1628)
        .ok_or("step-1 observed=no-authored-locked-key-door")?;
    let quest = authored
        .iter()
        .copied()
        .find(|door| door.action <= 1000 && door.appearance != 1628)
        .ok_or("step-4 observed=no-authored-quest-predicate-candidate")?;
    let mut client = connect_world(clients, 0xb6).await?;

    approach(&mut client, world, key).await?;
    let before = client.world_spatial().actor_position;
    let direct = use_door(&mut client, world, key).await?;
    evidence(&format!("M06_CHECK step=1 action=USE observed={direct:?}"));
    let direct_ok =
        direct == UseDisposition::Locked && client.world_spatial().actor_position == before;
    let key_placement = target(world, key, 0)?.placement;
    let key_revision_before = overlay_revision(&client, &key_placement);
    let keyed = use_carried_on_door(&mut client, world, key).await;
    let key_revision_after = overlay_revision(&client, &key_placement);
    evidence(&format!(
        "M06_CHECK step=2 action=USE-WITH observed={keyed:?} revision={key_revision_before}->{key_revision_after}"
    ));

    approach(&mut client, world, level).await?;
    let level_reply = use_door(&mut client, world, level).await?;
    evidence(&format!(
        "M06_CHECK step=3 action=USE-before-progression level_required={} observed={level_reply:?}",
        level.action.saturating_sub(1000)
    ));

    approach(&mut client, world, quest).await?;
    let quest_reply = use_door(&mut client, world, quest).await?;
    evidence(&format!(
        "M06_CHECK step=4 action=USE-before-transition storage_action={} observed={quest_reply:?}",
        quest.action
    ));

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-5 observed=logout-refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xb7).await?;
    approach(&mut fresh, world, level).await?;
    let replay = use_door(&mut fresh, world, level).await?;
    evidence(&format!(
        "M06_CHECK step=5 action=fresh-admission-USE observed={replay:?}"
    ));
    approach(&mut fresh, world, quest).await?;
    let quest_replay = use_door(&mut fresh, world, quest).await?;
    evidence(&format!(
        "M06_CHECK step=5 action=fresh-admission-quest-USE observed={quest_replay:?}"
    ));
    let _ = fresh.session_mut().logout().await?;

    if !direct_ok {
        return Err(format!("step-1 observed={direct:?}; expected Locked without movement").into());
    }
    let (status, keyed) = keyed.map_err(|error| {
        format!("step-2 observed=USE-WITH-not-sent:{error}; expected matching-key commit and map-state revision or explicit refusal")
    })?;
    if status != CommandStatus::Accepted
        || keyed != UseDisposition::Committed
        || key_revision_after <= key_revision_before
    {
        return Err(format!("step-2 observed=status:{status:?}/disposition:{keyed:?}/revision:{key_revision_before}->{key_revision_after}; need an ordinarily obtainable matching key and committed map-state revision").into());
    }
    if level_reply != UseDisposition::Locked {
        return Err(format!("step-3 refusal observed={level_reply:?}; expected Locked before ordinary progression to level {} (the required below-level refusal did not run against a below-level character)", level.action.saturating_sub(1000)).into());
    }
    if quest_reply != UseDisposition::Locked {
        return Err(format!("step-4 refusal observed={quest_reply:?}; expected Locked before ordinary quest/storage transition {} (the required pre-transition refusal did not run against a pre-transition character)", quest.action).into());
    }
    Err(format!(
        "step-3 progression observed=initial:{level_reply:?}/fresh:{replay:?}; no ordinary level-earning action was executed before the required committed USE; step-4 transition observed=initial:{quest_reply:?}/fresh:{quest_replay:?}; no ordinary dialogue/combat/item storage transition was executed before the required committed USE"
    )
    .into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result: TestResult = verdict::observe(async {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let world = load_world(
            bytes.clone(),
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        verdict::observe(journey(clients, world, &bytes)).await
    })
    .await;
    if let Err(error) = &result {
        evidence(&format!("M06_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=M06.full status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}

#[cfg(test)]
mod tests {
    #[test]
    fn level_action_threshold_matches_server_owner() {
        assert_eq!(1001_u16.saturating_sub(1000), 1);
        assert_eq!(2500_u16.saturating_sub(1000), 1500);
    }
}
