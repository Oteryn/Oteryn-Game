//! W01a replay: a rat's chase, damage and flight applied through `Session::apply_state_delta`
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

struct Segment {
    snapshot: Vec<u8>,
    deltas: Vec<Vec<u8>>,
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

type Fixture = (String, Vec<(&'static str, Segment)>);

fn fixture() -> Result<Fixture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("w01a-expected.txt"))?;
        let mut segments = Vec::new();
        for name in ["chase", "fight"] {
            let count = value(&expected, &format!("{name}.trace"))?
                .split(';')
                .count();
            let mut deltas = Vec::new();
            for index in 0..count {
                deltas.push(std::fs::read(
                    root.join(format!("w01a-{name}-delta{index}.bin")),
                )?);
            }
            segments.push((
                name,
                Segment {
                    snapshot: std::fs::read(root.join(format!("w01a-{name}-snapshot.bin")))?,
                    deltas,
                },
            ));
        }
        return Ok((expected, segments));
    }
    // Inline fixture: the rat at (14, 10) chases to (12, 10); fighting takes it to 20% and it flees.
    let rat = |x, health| {
        let mut entity = actor(
            EntityKind::Creature,
            8,
            1,
            ActorPosition { x, y: 10, floor: 0 },
        );
        entity.detail = Detail::Actor {
            direction: StepDirection::West,
            appearance_ref: 1,
            health_percent: health,
        };
        entity
    };
    let header = ActorPosition {
        x: 10,
        y: 10,
        floor: 0,
    };
    let snapshot = |rat_x, health| {
        let mut own_entity = own(header);
        own_entity.position = header;
        encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: header,
            own_identity: OWN,
            entities: vec![own_entity, rat(rat_x, health)],
        })
        .map_err(|error| format!("{error:?}"))
    };
    let delta = |entity| {
        encode_world_spatial_entities_delta(&entities_delta(header, vec![], vec![entity], vec![]))
            .map_err(|error| format!("{error:?}"))
    };
    let target = format!("{}:1", "08".repeat(16));
    let expected = format!(
        "chase.target={target}\nchase.initial=14,10,0|100\nchase.trace=13,10,0|100;12,10,0|100\n\
         fight.target={target}\nfight.initial=12,10,0|100\nfight.trace=12,10,0|60;12,10,0|20;13,10,0|20\n\
         chase.final=12,10,0\nlow=20\nflee.final=13,10,0\n"
    );
    Ok((
        expected,
        vec![
            (
                "chase",
                Segment {
                    snapshot: snapshot(14, 100)?,
                    deltas: vec![delta(rat(13, 100))?, delta(rat(12, 100))?],
                },
            ),
            (
                "fight",
                Segment {
                    snapshot: snapshot(12, 100)?,
                    deltas: vec![
                        delta(rat(12, 60))?,
                        delta(rat(12, 20))?,
                        delta(rat(13, 20))?,
                    ],
                },
            ),
        ],
    ))
}

#[test]
fn e2e_w01a_rat_chase_damage_and_flight_replay() -> Result<(), BoxError> {
    let (expected, segments) = fixture()?;
    block_on(async {
        let mut finals = Vec::new();
        for (name, segment) in segments {
            let target = parse_target(value(&expected, &format!("{name}.target"))?)?;
            let trace: Vec<&str> = value(&expected, &format!("{name}.trace"))?
                .split(';')
                .collect();
            assert_eq!(trace.len(), segment.deltas.len());
            if name == "fight" {
                let low = value(&expected, "low")?;
                assert!(trace.iter().any(|line| line.ends_with(&format!("|{low}"))));
            }
            let (client, peer) = entity_peer(
                (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, segment.snapshot),
                &[6],
                vec![],
            );
            let mut session = Session::admit(client, entity_admission()?).await?;
            let view = session.world_entities().ok_or("capability 6 selected")?;
            assert_eq!(
                state_text(view.get(&target)),
                value(&expected, &format!("{name}.initial"))?
            );
            let mut revision = session.join_snapshot().world_spatial_revision;
            let mut last = state_text(view.get(&target));
            for (index, (bytes, line)) in (0_u64..).zip(segment.deltas.iter().zip(&trace)) {
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
                let now = state_text(
                    session
                        .world_entities()
                        .ok_or("capability 6 selected")?
                        .get(&target),
                );
                if *line != "same" {
                    last = (*line).to_owned();
                }
                assert_eq!(now, last);
            }
            finals.push((name, last));

            // A garbage entity payload is refused whole: the view stays as applied.
            let before: Vec<_> = session
                .world_entities()
                .ok_or("capability 6 selected")?
                .iter()
                .copied()
                .collect();
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
        }
        let position = |text: &str| text.split('|').next().unwrap_or_default().to_owned();
        assert_eq!(position(&finals[0].1), value(&expected, "chase.final")?);
        assert_eq!(position(&finals[1].1), value(&expected, "flee.final")?);
        Ok(())
    })?
}
