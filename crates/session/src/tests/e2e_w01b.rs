//! W01b replay: a rat's kill (damage, removal, corpse) and its authored respawn applied through `Session::apply_state_delta`
//! (the domain-1 type-2 entity decode in `domain_dispatch.rs`) and exposed by
//! `Session::world_entities`, the entity view the world renderer reads.
use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail as Detail, encode_world_spatial_entities_delta,
};
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, BoxError> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

struct Fixture {
    expected: String,
    kill_snapshot: Vec<u8>,
    kill_deltas: Vec<Vec<u8>>,
    respawn_snapshot: Vec<u8>,
}

fn state_text(entity: Option<&WorldSpatialEntity>) -> String {
    let Some(entity) = entity else {
        return "absent".into();
    };
    let health = match entity.detail {
        Detail::Actor { health_percent, .. } => health_percent.to_string(),
        _ => "-".into(),
    };
    format!(
        "{},{},{}|{health}",
        entity.position.x, entity.position.y, entity.position.floor
    )
}

fn parse_target(text: &str) -> Result<EntityRef, BoxError> {
    let (identity, generation) = text.split_once(':').ok_or("target lacks generation")?;
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&identity[index * 2..index * 2 + 2], 16)?;
    }
    Ok(EntityRef {
        identity: bytes,
        generation: generation.parse()?,
    })
}

fn fixture() -> Result<Fixture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("w01b-expected.txt"))?;
        let count = value(&expected, "kill.trace")?.split(';').count();
        let mut kill_deltas = Vec::new();
        for index in 0..count {
            kill_deltas.push(std::fs::read(
                root.join(format!("w01b-kill-delta{index}.bin")),
            )?);
        }
        return Ok(Fixture {
            kill_snapshot: std::fs::read(root.join("w01b-kill-snapshot.bin"))?,
            respawn_snapshot: std::fs::read(root.join("w01b-respawn-snapshot.bin"))?,
            kill_deltas,
            expected,
        });
    }
    // Inline fixture: the rat at (14, 10) drops to 40%, dies and leaves a corpse; a replacement
    // generation stands at the authored point after the respawn interval.
    let header = ActorPosition {
        x: 10,
        y: 10,
        floor: 0,
    };
    let at_x = |x| ActorPosition { x, y: 10, floor: 0 };
    let rat = |generation, health| {
        let mut entity = actor(EntityKind::Creature, 8, generation, at_x(14));
        entity.detail = Detail::Actor {
            direction: StepDirection::West,
            appearance_ref: 1,
            health_percent: health,
        };
        entity
    };
    let snapshot = |others: Vec<WorldSpatialEntity>| {
        let mut entities = vec![own(header)];
        entities.extend(others);
        encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: header,
            own_identity: OWN,
            entities,
        })
        .map_err(|error| format!("{error:?}"))
    };
    let delta = |enter, update, leave| {
        encode_world_spatial_entities_delta(&entities_delta(header, enter, update, leave))
            .map_err(|error| format!("{error:?}"))
    };
    let dead = rat(1, 100).entity;
    let corpse_entity = corpse(9, at_x(14));
    let expected = format!(
        "kill.target={}:1\nkill.initial=14,10,0|100\nkill.trace=14,10,0|40;absent\n\
         corpse={}:0\ncorpse.position=14,10,0\nspawn=14,10,0\n\
         replacement={}:2\nreplacement.position=14,10,0\nkilled.absent=true\n",
        "08".repeat(16),
        "09".repeat(16),
        "08".repeat(16)
    );
    Ok(Fixture {
        kill_snapshot: snapshot(vec![rat(1, 100)])?,
        kill_deltas: vec![
            delta(vec![], vec![rat(1, 40)], vec![])?,
            delta(vec![corpse_entity], vec![], vec![dead])?,
        ],
        respawn_snapshot: snapshot(vec![corpse_entity, rat(2, 100)])?,
        expected,
    })
}

fn at_text(text: &str) -> Result<ActorPosition, BoxError> {
    let mut parts = text.split(',');
    let mut next = || parts.next().ok_or("position field absent");
    Ok(ActorPosition {
        x: next()?.parse()?,
        y: next()?.parse()?,
        floor: next()?.parse()?,
    })
}

#[test]
fn e2e_w01b_rat_kill_and_respawn_replay() -> Result<(), BoxError> {
    let fixture = fixture()?;
    let expected = &fixture.expected;
    let target = parse_target(value(expected, "kill.target")?)?;
    let corpse_ref = parse_target(value(expected, "corpse")?)?;
    let replacement = parse_target(value(expected, "replacement")?)?;
    let spawn = at_text(value(expected, "spawn")?)?;
    block_on(async {
        // Kill: the damage delta, then the removal with the corpse.
        let trace: Vec<&str> = value(expected, "kill.trace")?.split(';').collect();
        assert_eq!(trace.len(), fixture.kill_deltas.len());
        let (client, peer) = entity_peer(
            (
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                fixture.kill_snapshot.clone(),
            ),
            &[6],
            vec![],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert_eq!(
            state_text(
                session
                    .world_entities()
                    .ok_or("capability 6 selected")?
                    .get(&target)
            ),
            value(expected, "kill.initial")?
        );
        let mut revision = session.join_snapshot().world_spatial_revision;
        let mut last = value(expected, "kill.initial")?.to_owned();
        for (index, (bytes, line)) in (0_u64..).zip(fixture.kill_deltas.iter().zip(&trace)) {
            let frame = encode_state_delta(
                1,
                41 + index,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision,
                revision + 1,
                DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                bytes,
            )?;
            session.apply_state_delta(41 + index, decode_wire_envelope(&frame)?.payload())?;
            revision += 1;
            if *line != "same" {
                last = (*line).to_owned();
            }
            assert_eq!(
                state_text(
                    session
                        .world_entities()
                        .ok_or("capability 6 selected")?
                        .get(&target)
                ),
                last
            );
        }
        let view = session.world_entities().ok_or("capability 6 selected")?;
        assert!(view.get(&target).is_none());
        let corpse = view
            .get(&corpse_ref)
            .ok_or("corpse absent after the kill")?;
        assert_eq!(corpse.kind, EntityKind::Corpse);
        assert_eq!(
            corpse.position,
            at_text(value(expected, "corpse.position")?)?
        );

        // A garbage entity payload is refused whole: the view stays as applied.
        let before: Vec<_> = view.iter().copied().collect();
        let frame = encode_state_delta(
            1,
            99,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision,
            revision + 1,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(99, decode_wire_envelope(&frame)?.payload())
                .is_err()
        );
        let after: Vec<_> = session
            .world_entities()
            .ok_or("capability 6 selected")?
            .iter()
            .copied()
            .collect();
        assert_eq!(after, before);
        drop(session);
        peer.await??;

        // Respawn: a later admission holds the replacement at the authored point and not the
        // killed generation.
        let (client, peer) = entity_peer(
            (
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                fixture.respawn_snapshot.clone(),
            ),
            &[6],
            vec![],
        );
        let session = Session::admit(client, entity_admission()?).await?;
        let view = session.world_entities().ok_or("capability 6 selected")?;
        let fresh = view.get(&replacement).ok_or("replacement absent")?;
        assert_eq!(fresh.kind, EntityKind::Creature);
        assert_eq!(fresh.position, spawn);
        assert_ne!(replacement, target);
        assert_eq!(value(expected, "killed.absent")?, "true");
        assert!(view.get(&target).is_none());
        drop(session);
        peer.await??;
        Ok(())
    })?
}
