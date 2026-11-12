//! C13 replay: summons/familiars and owner end-of-life.
//!
//! The consumer is `domain_dispatch.rs::Session::apply_domain_delta` (domain 1/type 2), which
//! decodes each payload and calls `entities.rs::WorldEntities::after`; the result is observed
//! through `session_access.rs::Session::world_entities`. A session that watches the owner first
//! holds the summon after the captured enter delta, then loses it (and the owner) through the
//! captured owner-end delta the server sent when the owner logged out.

use super::*;
use oteryn_protocol_oteryn::entity_names::EntityDisplayName;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLights, decode_world_spatial_entities_delta_with_fields,
    decode_world_spatial_entities_snapshot_with_fields,
    encode_world_spatial_entities_delta_with_fields,
    encode_world_spatial_entities_snapshot_with_fields,
};

const C13_CAPABILITIES: &[u32] = &[6, 286];

struct Capture {
    baseline: Vec<u8>,
    spawn: Vec<u8>,
    ended: Vec<u8>,
    expected: String,
}

fn identity_hex(identity: &[u8; 16]) -> String {
    identity.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fixture(fields: EntityFields) -> Result<Capture, BoxError> {
    let mut owner = actor(EntityKind::Player, 0xb1, 3, at(0, 1));
    owner.display_name = Some(EntityDisplayName::new("Owner").map_err(|e| format!("name: {e:?}"))?);
    let mut me = own(at(0, 0));
    me.display_name = Some(EntityDisplayName::new("Observer").map_err(|e| format!("name: {e:?}"))?);
    let mut rat = actor(EntityKind::Creature, 0xc1, 7, at(1, 0));
    rat.display_name = Some(EntityDisplayName::new("Rat").map_err(|e| format!("name: {e:?}"))?);
    let baseline = WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT_GENERATION,
        actor_position: at(0, 0),
        own_identity: OWN,
        entities: vec![me, owner],
    };
    let spawn = entities_delta(at(0, 0), vec![rat], vec![], vec![]);
    let ended = entities_delta(at(0, 0), vec![], vec![], vec![rat.entity, owner.entity]);
    let lights = EntityLights::new();
    Ok(Capture {
        baseline: encode_world_spatial_entities_snapshot_with_fields(&baseline, fields, &lights)
            .map_err(|e| format!("fixture baseline: {e:?}"))?,
        spawn: encode_world_spatial_entities_delta_with_fields(&spawn, fields, &lights)
            .map_err(|e| format!("fixture spawn: {e:?}"))?,
        ended: encode_world_spatial_entities_delta_with_fields(&ended, fields, &lights)
            .map_err(|e| format!("fixture owner end: {e:?}"))?,
        expected: format!(
            "Rat\n{}\n{}\n{}\n",
            identity_hex(&rat.entity.identity),
            rat.entity.generation,
            identity_hex(&owner.entity.identity)
        ),
    })
}

fn load(fields: EntityFields) -> Result<Capture, BoxError> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture(fields);
    };
    let root = std::path::Path::new(&root);
    Ok(Capture {
        baseline: std::fs::read(root.join("c13-baseline.bin"))?,
        spawn: std::fs::read(root.join("c13-spawn.bin"))?,
        ended: std::fs::read(root.join("c13-owner-ended.bin"))?,
        expected: std::fs::read_to_string(root.join("c13-expected.txt"))?,
    })
}

#[test]
fn e2e_c13_replays_summon_and_owner_end() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(e2e_c13_scenario)?
        .join()
        .map_err(|_| "C13 replay thread panicked")?
}

fn e2e_c13_scenario() -> Result<(), BoxError> {
    block_on(async {
        let fields = EntityFields::selected(C13_CAPABILITIES);
        let capture = load(fields)?;
        let lines = capture.expected.lines().collect::<Vec<_>>();
        let [name, rat_hex, rat_generation, owner_hex] = lines.as_slice() else {
            return Err("invalid C13 expected capture".into());
        };
        let baseline =
            decode_world_spatial_entities_snapshot_with_fields(&capture.baseline, fields)
                .map_err(|e| format!("baseline payload: {e:?}"))?
                .0;
        let spawn = decode_world_spatial_entities_delta_with_fields(&capture.spawn, fields)
            .map_err(|e| format!("spawn payload: {e:?}"))?
            .0;
        let ended = decode_world_spatial_entities_delta_with_fields(&capture.ended, fields)
            .map_err(|e| format!("owner-end payload: {e:?}"))?
            .0;
        let rat = spawn
            .enter
            .iter()
            .find(|entity| entity.kind == EntityKind::Creature)
            .ok_or("captured C13 spawn delta has no creature")?;
        assert_eq!(identity_hex(&rat.entity.identity), *rat_hex);
        assert_eq!(rat.entity.generation.to_string(), *rat_generation);
        let owner = baseline
            .entities
            .iter()
            .find(|entity| identity_hex(&entity.entity.identity) == *owner_hex)
            .ok_or("owner absent from the C13 baseline")?
            .entity;
        assert!(ended.leave.contains(&rat.entity) && ended.leave.contains(&owner));

        let (client, peer) = entity_peer(
            (
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                capture.baseline.clone(),
            ),
            C13_CAPABILITIES,
            vec![],
        );
        let mut session = Session::admit(client, entity_admission()?)
            .await
            .map_err(|e| format!("C13 admission: {e:?}"))?;
        let held = baseline.entities.len();
        {
            let entities = session.world_entities().ok_or("entity store absent")?;
            assert_eq!(entities.len(), held);
            assert!(entities.get(&rat.entity).is_none());
            assert!(entities.get(&owner).is_some());
        }

        // The captured summon enter delta makes the session hold the Rat.
        let frame = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &capture.spawn,
        )?;
        session
            .apply_state_delta(41, decode_wire_envelope(&frame)?.payload())
            .map_err(|e| format!("spawn apply: {e:?}"))?;
        let stored = session
            .world_entities()
            .and_then(|entities| entities.get(&rat.entity))
            .ok_or("session does not hold the summon after the enter delta")?;
        assert_eq!(
            stored
                .display_name
                .and_then(|n| n.as_str().ok().map(str::to_owned))
                .as_deref(),
            Some(*name)
        );
        assert_eq!(stored.kind, EntityKind::Creature);
        assert_eq!(
            session.world_entities().map(|entities| entities.len()),
            Some(held + spawn.enter.len())
        );

        // A corrupt payload is refused and changes nothing.
        let before = session
            .world_entities()
            .cloned()
            .ok_or("entity store absent")?;
        let invalid = encode_state_delta(
            1,
            42,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            6,
            7,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(42, decode_wire_envelope(&invalid)?.payload())
                .is_err()
        );
        assert_eq!(session.world_entities(), Some(&before));

        // The captured owner-end delta removes the summon and the owner.
        let frame = encode_state_delta(
            1,
            42,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            6,
            7,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &capture.ended,
        )?;
        session
            .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
            .map_err(|e| format!("owner-end apply: {e:?}"))?;
        {
            let entities = session.world_entities().ok_or("entity store absent")?;
            assert!(entities.get(&rat.entity).is_none());
            assert!(entities.get(&owner).is_none());
            assert_eq!(
                entities.len(),
                held + spawn.enter.len() + ended.enter.len() - ended.leave.len()
            );
        }

        // Replaying the owner-end delta leaves an entity that is already gone: refused.
        let after = session
            .world_entities()
            .cloned()
            .ok_or("entity store absent")?;
        let replay = encode_state_delta(
            1,
            43,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            7,
            8,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &capture.ended,
        )?;
        assert!(
            session
                .apply_state_delta(43, decode_wire_envelope(&replay)?.payload())
                .is_err()
        );
        assert_eq!(session.world_entities(), Some(&after));
        drop(session);
        peer.await??;
        Ok(())
    })?
}
