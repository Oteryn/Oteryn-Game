//! Client replay capture for SCENARIO_MATRIX C13 (summons and owner end-of-life).
//!
//! An owner summons a Rat while a second character observes. The observer's session applies the
//! server's WORLD_SPATIAL_ENTITIES deltas: the one that brings the summon in, and the one that
//! takes it out again when the owner logs out. Both applied deltas are captured; the observer's
//! pre-summon entity store is captured only as the starting point to replay them from.
//! The consumer is `crates/session/src/domain_dispatch.rs::Session::apply_domain_delta`
//! (domain 1/type 2), which calls `crates/session/src/entities.rs::WorldEntities::after` and is
//! exposed by `Session::world_entities`.

use super::*;
use oteryn_dev_client::{EntityKind, EntityRef, SessionEvent, WorldSpatialEntitiesDelta};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLights, WorldSpatialEntitiesSnapshot,
    encode_world_spatial_entities_delta_with_fields,
    encode_world_spatial_entities_snapshot_with_fields,
};

const FIELD_CAPABILITIES: &[u32] = &[6, 286];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(capture(clients)).await;
    match result.unwrap_or_else(|_| Err("C13.client journey timeout".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=C13.client status=PASS evidence=observer-summon-enter-and-owner-end-leave",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C13.client status=FAIL evidence={error}"
        )),
    }
}

fn is_rat(entity: &oteryn_dev_client::WorldSpatialEntity) -> bool {
    entity.kind == EntityKind::Creature
        && entity
            .display_name
            .as_ref()
            .is_some_and(|name| name.as_str() == Ok("Rat"))
}

/// The next entity delta the observer applied that satisfies `wanted`.
async fn next_delta(
    observer: &mut oteryn_dev_client::DevClientSession,
    what: &str,
    wanted: impl Fn(&WorldSpatialEntitiesDelta) -> bool,
) -> TestResult<WorldSpatialEntitiesDelta> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        for event in observer.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && wanted(&delta.value)
            {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C13.client observer never applied {what}").into());
        }
        observer.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut owner = connect_world(clients, 0x2d).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
        let mut observer = connect_character(clients, 0x2e, 0).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            if !FIELD_CAPABILITIES
                .iter()
                .all(|capability| observer.selected_capabilities().contains(capability))
            {
                return Err("C13.client observer lacks entity capabilities 6 and 286".into());
            }
            let from = tile(owner.world_spatial().actor_position)?;
            let path = route(world, from, |at| {
                world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|entry| !entry.protection_zone())
            })
            .ok_or("C13.client has no route outside protection zone")?;
            walk(&mut owner, world, &path).await?;
            let spot = tile(owner.world_spatial().actor_position)?;
            let near = |at: TilePos| {
                at != spot
                    && at.floor == spot.floor
                    && at.x.abs_diff(spot.x).max(at.y.abs_diff(spot.y)) <= 2
            };
            let approach = route(world, tile(observer.world_spatial().actor_position)?, near)
                .ok_or("C13.client observer has no route next to the owner")?;
            walk(&mut observer, world, &approach).await?;
            owner.take_events();
            observer
                .service_liveness(Duration::from_millis(100))
                .await?;
            observer.take_events();

            let reply = owner
                .chat(&oteryn_dev_client::client_runtime::chat::ChatIntent::Say {
                    mode: oteryn_dev_client::client_runtime::chat::ChatSpeechMode::Say,
                    text: "utevo res \"rat\"".into(),
                })
                .await?;
            if reply.status != CommandStatus::Accepted {
                return Err(format!("C13.client summon refused: {:?}", reply.disposition).into());
            }
            let spawn = next_delta(&mut observer, "the summon enter delta", |delta| {
                delta.enter.iter().any(is_rat)
            })
            .await?;
            let rat = *spawn
                .enter
                .iter()
                .find(|entity| is_rat(entity))
                .ok_or("C13.client summon missing from the captured delta")?;
            if rat.entity.generation == 0 || !spawn.leave.is_empty() {
                return Err("C13.client spawn delta is not an isolated summon enter".into());
            }
            // Replay start: the observer's store with the summon taken out again.
            let entities = observer
                .world_entities()
                .ok_or("C13.client observer entity store absent")?;
            let baseline = WorldSpatialEntitiesSnapshot {
                content_generation: spawn.content_generation,
                actor_position: spawn.actor_position,
                own_identity: *entities.own_identity(),
                entities: entities
                    .iter()
                    .filter(|entity| entity.entity != rat.entity)
                    .copied()
                    .collect(),
            };
            let owner_entity = baseline
                .entities
                .iter()
                .find(|entity| {
                    entity.kind == EntityKind::Player
                        && entity.entity.identity != baseline.own_identity
                })
                .map(|entity| entity.entity)
                .ok_or("C13.client observer does not see the owner")?;

            if owner.session_mut().logout().await?.result
                != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
            {
                return Err("C13.client owner logout refused".into());
            }
            row_needs_logout_0 = false;
            let ended = next_delta(&mut observer, "the owner-end leave delta", |delta| {
                delta.leave.contains(&rat.entity)
            })
            .await?;
            let stored: Vec<EntityRef> = baseline
                .entities
                .iter()
                .map(|entity| entity.entity)
                .chain([rat.entity])
                .collect();
            if !ended.leave.contains(&owner_entity)
                || !ended.leave.iter().all(|leaving| stored.contains(leaving))
                || ended
                    .update
                    .iter()
                    .any(|updated| !stored.contains(&updated.entity))
            {
                return Err("C13.client owner-end delta is not isolated from other changes".into());
            }
            if observer
                .world_entities()
                .is_some_and(|entities| entities.get(&rat.entity).is_some())
            {
                return Err("C13.client observer still holds the summon after owner end".into());
            }

            let fields = EntityFields::selected(FIELD_CAPABILITIES);
            let lights = EntityLights::new();
            let encoded_baseline =
                encode_world_spatial_entities_snapshot_with_fields(&baseline, fields, &lights)
                    .map_err(|error| format!("C13.client baseline codec: {error:?}"))?;
            let encoded_spawn =
                encode_world_spatial_entities_delta_with_fields(&spawn, fields, &lights)
                    .map_err(|error| format!("C13.client spawn codec: {error:?}"))?;
            let encoded_ended =
                encode_world_spatial_entities_delta_with_fields(&ended, fields, &lights)
                    .map_err(|error| format!("C13.client owner-end codec: {error:?}"))?;
            let hex = |identity: &[u8; 16]| {
                identity
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                let path = Path::new(&path);
                std::fs::write(path.join("c13-baseline.bin"), &encoded_baseline)?;
                std::fs::write(path.join("c13-spawn.bin"), &encoded_spawn)?;
                std::fs::write(path.join("c13-owner-ended.bin"), &encoded_ended)?;
                std::fs::write(
                    path.join("c13-expected.txt"),
                    format!(
                        "Rat\n{}\n{}\n{}\n",
                        hex(&rat.entity.identity),
                        rat.entity.generation,
                        hex(&owner_entity.identity)
                    ),
                )?;
            }
            row_logout(&mut observer).await?;
            row_needs_logout_1 = false;
            Ok(())
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            row_logout(&mut observer).await?;
        }
        row_result
    }
    .await;
    if row_needs_logout_0 {
        row_logout(&mut owner).await?;
    }
    row_result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rat_requires_both_creature_kind_and_exact_display_name() -> TestResult {
        let mut entity = oteryn_dev_client::WorldSpatialEntity {
            display_name: Some(
                oteryn_protocol_oteryn::entity_names::EntityDisplayName::new("Rat")
                    .map_err(|error| format!("{error:?}"))?,
            ),
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            detail: oteryn_dev_client::EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        assert!(is_rat(&entity));
        entity.kind = EntityKind::Player;
        assert!(!is_rat(&entity));
        entity.kind = EntityKind::Creature;
        entity.display_name = None;
        assert!(!is_rat(&entity));
        Ok(())
    }
}
