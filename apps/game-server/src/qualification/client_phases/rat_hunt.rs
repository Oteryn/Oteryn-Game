//! Ordinary rat advancement: server chase, native XP and fresh corpse evidence.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{attack, cyclopedia_character as stats};
#[path = "hunt_observation.rs"]
mod observation;
use observation::HuntObservation;

async fn experience(client: &mut DevClientSession) -> TestResult<u64> {
    let tab = stats::CharacterTab::GeneralStats as u8;
    let result = client
        .session_mut()
        .gated_command(
            stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[tab, 0, 0, 50],
        )
        .await?;
    if result.status != CommandStatus::Accepted || result.payload.first() != Some(&tab) {
        return Err(format!("rat hunt GeneralStats refused: {result:?}").into());
    }
    Ok(u64::from_le_bytes(
        result
            .payload
            .get(1..9)
            .ok_or("rat hunt XP frame truncated")?
            .try_into()?,
    ))
}

pub(crate) async fn hunt_rat(
    client: &mut DevClientSession,
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    let name = client_world::native_creature_name("oteryn:creature.rat")?;
    let origin = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry
                        .display_name
                        .is_some_and(|value| value.as_str() == Ok(name))
                    && entry.position.floor == origin.floor
                    && entry
                        .position
                        .x
                        .abs_diff(origin.x)
                        .max(entry.position.y.abs_diff(origin.y))
                        <= 1
            })
        })
        .copied()
        .ok_or_else(|| format!("rat hunt has no verified adjacent {name} at {origin:?}"))?;
    let modes = attack::FightModes {
        chase: attack::ChaseMode::Chase,
        ..attack::FightModes::DEFAULT
    };
    let reply = client.session_mut().set_fight_modes(&modes).await?;
    if reply.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!("rat hunt chase refused: {reply:?}").into());
    }
    let xp_before = experience(client).await?;
    let target = client
        .world_entities()
        .and_then(|entities| entities.get(&target.entity))
        .copied()
        .ok_or("rat left visibility before ATTACK")?;
    let excluded = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    let mut observed = HuntObservation::new(target, excluded);
    let mut actor_position = client.world_spatial().actor_position;
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    evidence(&format!(
        "rat_hunt phase=attack start={origin:?} target={:?} target_position={:?} name={name:?} modes={modes:?} reply={attack:?} xp_before={xp_before}",
        target.entity, target.position
    ));
    if attack.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!("rat hunt ATTACK refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    client_world::hunt_observation_wait(deadline);
    let mut xp_after = xp_before;
    let mut last_xp_query: Option<tokio::time::Instant> = None;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                let next = delta.value.actor_position;
                let jumped = actor_position
                    .x
                    .abs_diff(next.x)
                    .max(actor_position.y.abs_diff(next.y))
                    > 8;
                actor_position = next;
                observed.observe(&delta.value);
                if jumped {
                    client.session_mut().attack_target(None).await?;
                    return Err(format!("rat hunt interrupted: actor relocated before kill proof; start={origin:?} actual={actor_position:?} target={:?} observation={observed:?} vitals={:?} xp_before={xp_before} xp_after={xp_after}",target.entity,client.actor_vitals()).into());
                }
            } else if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health == 0
            {
                client.session_mut().attack_target(None).await?;
                return Err(format!("rat hunt player died before kill proof; start={origin:?} observation={observed:?}").into());
            }
        }
        let alive = client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_some());
        if !alive
            && observed.removed
            && last_xp_query.is_none_or(|last| last.elapsed() >= Duration::from_millis(250))
        {
            xp_after = experience(client).await?;
            last_xp_query = Some(tokio::time::Instant::now());
            // The query can service more spatial frames. Observe their wire order
            // before deciding whether a corpse belongs to this removed target.
            continue;
        }
        if !alive
            && let Some(corpse) = client
                .world_entities()
                .and_then(|entities| observed.corpse(entities.iter(), xp_after > xp_before))
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err(format!("rat hunt produced no proven corpse before deadline; start={origin:?} actual={actor_position:?} target={:?} target_present={alive} observation={observed:?} xp_before={xp_before} xp_after={xp_after} vitals={:?}",target.entity,client.actor_vitals()).into());
        }
    };
    evidence(&format!(
        "rat_hunt phase=kill target={:?} corpse={:?} xp_before={xp_before} xp_after={xp_after}",
        target.entity, corpse.entity
    ));
    client.session_mut().attack_target(None).await?;
    client_world::approach_hunt_corpse(client, world, corpse.position).await?;
    loot_corpse(client, target, corpse).await
}
