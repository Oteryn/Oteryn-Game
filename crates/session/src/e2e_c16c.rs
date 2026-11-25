//! C16c client summon replay.
//!
//! c16c-snapshot.bin is the unmodified world-entities snapshot payload.
//! c16c-deltas.bin starts with ASCII OTR1, a u64 baseline server sequence, two
//! counted u32 lists (selected capabilities and appearance item references),
//! and a u32 snapshot count. Each snapshot entry is a u32 domain, u64 revision,
//! u32 snapshot type and length-prefixed payload. The world entry has length
//! zero and uses c16c-snapshot.bin instead. Each remaining record until EOF is
//! a length-prefixed StateDelta envelope in original server-sequence order,
//! with generation 1 and the original domain, revisions, type and payload.
//! All lengths and list counts are u32 and integers are little-endian.
//! c16c-expected.txt is "32 hex identity digits, decimal generation, decimal
//! EntityKind discriminant", separated by spaces, followed by a newline.

use super::{replay_input::*, *};
use replay_capture::C16C_FILES;
use world_spatial_entities::{
    EntityFields, EntityLights, encode_world_spatial_entities_delta_with_fields,
    encode_world_spatial_entities_snapshot_with_fields,
};

#[test]
fn e2e_c16c_summon_delta_replay() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| block_on(replay_summon())?)?
        .join()
        .map_err(|_| "C16c replay thread")?
}

async fn replay_summon() -> Result<(), BoxError> {
    let fixture = summon_fixture()?;
    let mut session = fixture.admit().await?;
    let corrupt = encode_state_delta(1, 42, WORLD, 5, 6, 2, &[0])?;
    assert!(matches!(
        apply_frame(&mut session, &corrupt),
        Err(SessionError::WorldSpatial(_))
    ));
    assert!(session.raw_world_entity_deltas().is_empty());

    assert!(
        session
            .world_entities()
            .ok_or("C16c entities absent")?
            .get(&fixture.entity)
            .is_none()
    );
    assert_summon(&fixture, &mut session)?;
    let raw = session.raw_world_entity_deltas();
    assert_eq!(raw.len(), 1);
    let envelope = decode_wire_envelope(&fixture.deltas[0])?;
    let original = decode_state_delta(envelope.payload())?;
    assert_eq!(raw[0].payload.as_ref(), original.payload);
    assert_eq!(raw[0].server_sequence, envelope.server_sequence());
    assert_eq!(raw[0].delta_type, original.delta_type);

    let files = session
        .c16c_capture_files(fixture.entity)
        .ok_or("C16c capture missing")?;
    assert_eq!(files[0].1, fixture.snapshots[0].payload);
    let input = round_trip(files, C16C_FILES, WORLD)?;
    assert_eq!(input.deltas, fixture.deltas);
    assert_summon(&input, &mut input.admit().await?)?;

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let input = ReplayInput::read(std::path::Path::new(&path), C16C_FILES, WORLD)?;
        assert_summon(&input, &mut input.admit().await?)?;
    }
    Ok(())
}

fn assert_summon(input: &ReplayInput, session: &mut Session<DuplexStream>) -> Result<(), BoxError> {
    let events = input.apply(session)?;
    assert!(events.iter().any(|event| matches!(
        event, SessionEvent::WorldSpatialEntities(delta)
            if delta.value.enter.iter().any(|e| e.entity == input.entity && e.kind as u8 == input.expected)
    )));
    let entity = session
        .world_entities()
        .and_then(|world| world.get(&input.entity))
        .ok_or("C16c summon not visible after replay")?;
    assert_eq!(entity.kind, EntityKind::Creature);
    assert_eq!(entity.kind as u8, input.expected);
    Ok(())
}

fn summon_fixture() -> Result<ReplayInput, BoxError> {
    use oteryn_protocol_oteryn::world_clock::{self, WorldClock};
    let selected = vec![4, 6, 19, 286];
    let fields = EntityFields::selected(&selected);
    let lights = EntityLights::new();
    let mut summon = actor(EntityKind::Creature, 0xb0, 1, at(1, 0));
    summon.display_name =
        Some(EntityDisplayName::new("Summoned Rat").map_err(|e| format!("{e:?}"))?);
    let mut player = own(at(0, 0));
    player.display_name = Some(EntityDisplayName::new("Summoner").map_err(|e| format!("{e:?}"))?);
    let snapshot = WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT_GENERATION,
        actor_position: at(0, 0),
        own_identity: OWN,
        entities: vec![player],
    };
    let payload = encode_world_spatial_entities_snapshot_with_fields(&snapshot, fields, &lights)
        .map_err(|e| format!("{e:?}"))?;
    let delta = entities_delta(at(0, 0), vec![summon], vec![], vec![]);
    let change = encode_world_spatial_entities_delta_with_fields(&delta, fields, &lights)
        .map_err(|e| format!("{e:?}"))?;
    let clock = world_clock::encode_world_clock(WorldClock {
        tibian_minute: 0,
        light_level: 40,
        light_color: 215,
    })
    .map_err(|e| format!("{e:?}"))?;
    Ok(ReplayInput {
        sequence: 40,
        selected,
        refs: vec![],
        snapshots: vec![
            GatedSnapshot {
                domain_id: WORLD,
                revision: 5,
                snapshot_type: 2,
                payload,
            },
            GatedSnapshot {
                domain_id: world_clock::STATE_DOMAIN_WORLD_CLOCK,
                revision: 3,
                snapshot_type: world_clock::SNAPSHOT_TYPE_WORLD_CLOCK_V1,
                payload: clock,
            },
        ],
        deltas: vec![encode_state_delta(1, 42, WORLD, 5, 6, 2, &change)?],
        entity: summon.entity,
        expected: summon.kind as u8,
    })
}
