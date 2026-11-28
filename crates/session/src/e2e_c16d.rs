//! C16d client replay.
//!
//! c16d-entities.bin is the exact retained domain-1 payload.
//! c16d-expected.txt names the independently authored item definition as item:<reference>.
//! c16d-capabilities.bin is the snapshot prefix of OTR1, ending after the last snapshot.
//! After ASCII OTR1 come the u64 server sequence, two counted u32 lists (selected
//! capabilities and appearance item references), and a u32 snapshot count. Each snapshot
//! entry contains its u32 domain, u64 revision, u32 snapshot type and length-prefixed payload.
//! Domain 1 has length zero and uses c16d-entities.bin. Companion snapshots are inline.
//! All integers are little-endian. Counts and lengths are u32.
//!
//! `Session::admit` decodes the payload and `Session::world_entities` exposes the reference.

use super::{
    appearance_session_tests::appearance_fixture,
    replay_input::{APPEARANCE, SnapshotTranscript, WORLD, admit_snapshots},
    *,
};
use oteryn_protocol_oteryn::{actor_motion, appearance, entity_names, world_clock};
use replay_capture::C16D_FILES;
use world_spatial_entities::{
    EntityDetail, EntityFields, EntityLight, EntityLights, WorldSpatialEntitiesSnapshot,
    decode_world_spatial_entities_snapshot_with_fields,
    encode_world_spatial_entities_snapshot_with_fields,
};

#[test]
fn e2e_c16d_item_appearance_snapshot_replay() -> Result<(), BoxError> {
    block_on(async {
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            let [payload, expected, metadata] =
                C16D_FILES.map(|name| std::fs::read(path.join(name)));
            let expected = expected?;
            let item = std::str::from_utf8(&expected)?
                .trim()
                .strip_prefix("item:")
                .ok_or("C16d live replay requires an independent item expectation")?
                .parse()?;
            let input = decode_transcript(&payload?, &metadata?)?;
            assert_replay(input, ExpectedAppearance::Item(item)).await?;
        } else {
            let expected = 2_854;
            let mut actor = own(at(0, 0));
            actor.detail = EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: expected,
                health_percent: 100,
            };
            let selected = vec![CAPABILITY_WORLD_SPATIAL_ENTITIES];
            let world = world_fixture(actor, &selected, &EntityLights::new())?;
            assert_replay(
                SnapshotTranscript {
                    sequence: 40,
                    selected,
                    refs: vec![],
                    snapshots: vec![world],
                },
                ExpectedAppearance::Reference(expected),
            )
            .await?;
        }
        Ok(())
    })?
}

#[test]
fn e2e_c16d_capture_round_trip_with_negotiated_entity_fields() -> Result<(), BoxError> {
    block_on(async {
        let (appearance, mut actor, _) = appearance_fixture::snapshot()?;
        let expected = appearance.bindings[0].appearance_ref;
        actor.display_name = Some(
            entity_names::EntityDisplayName::new("C16d actor")
                .map_err(|error| format!("{error:?}"))?,
        );
        let selected = vec![
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            world_spatial::CAPABILITY_PACED_MOVEMENT_V1,
            world_clock::CAPABILITY_WORLD_LIGHT_V1,
            appearance::CAPABILITY_APPEARANCE_V1,
            entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2,
            actor_motion::CAPABILITY_ACTOR_MOTION_V1,
        ];
        let lights = EntityLights::from([(
            OWN,
            EntityLight {
                level: 8,
                color: 215,
            },
        )]);
        let world = world_fixture(actor, &selected, &lights)?;
        let snapshots = vec![
            world,
            GatedSnapshot {
                domain_id: APPEARANCE,
                revision: 1,
                snapshot_type: appearance::SNAPSHOT_TYPE_APPEARANCE,
                payload: appearance.encode().map_err(|error| format!("{error:?}"))?,
            },
            GatedSnapshot {
                domain_id: world_clock::STATE_DOMAIN_WORLD_CLOCK,
                revision: 1,
                snapshot_type: world_clock::SNAPSHOT_TYPE_WORLD_CLOCK_V1,
                payload: world_clock::encode_world_clock(world_clock::WorldClock {
                    tibian_minute: 60,
                    light_level: 128,
                    light_color: 215,
                })
                .map_err(|error| format!("{error:?}"))?,
            },
        ];
        let session = admit_snapshots(40, &selected, &snapshots, &[], CONTENT_GENERATION).await?;
        let files = session.c16d_capture_files().ok_or("C16d capture absent")?;
        assert_eq!(files.each_ref().map(|(name, _)| *name), C16D_FILES);
        let [(_, payload), (_, reference), (_, metadata)] = files;
        assert_eq!(payload, snapshots[0].payload);
        assert_eq!(reference, format!("{expected}\n").as_bytes());
        let (input, reference) = decode_capture(&payload, &reference, &metadata)?;
        assert_eq!(input.selected, selected);
        assert_eq!(input.snapshots, snapshots);
        let wrong = assert_replay(input.clone(), ExpectedAppearance::Item(2854)).await;
        assert_eq!(
            wrong
                .err()
                .ok_or("C16d wrong look passed replay")?
                .to_string(),
            "C16d replayed own look differs from expected item 2854"
        );
        assert_replay(input, ExpectedAppearance::Reference(reference)).await
    })?
}

fn world_fixture(
    actor: WorldSpatialEntity,
    selected: &[u32],
    lights: &EntityLights,
) -> Result<GatedSnapshot, BoxError> {
    let fixture = WorldSpatialEntitiesSnapshot {
        content_generation: CONTENT_GENERATION,
        actor_position: actor.position,
        own_identity: actor.entity.identity,
        entities: vec![actor],
    };
    let payload = encode_world_spatial_entities_snapshot_with_fields(
        &fixture,
        EntityFields::selected(selected),
        lights,
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok(GatedSnapshot {
        domain_id: WORLD,
        revision: 5,
        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        payload,
    })
}

fn decode_capture(
    payload: &[u8],
    expected: &[u8],
    metadata: &[u8],
) -> Result<(SnapshotTranscript, u32), BoxError> {
    Ok((
        decode_transcript(payload, metadata)?,
        std::str::from_utf8(expected)?.trim().parse()?,
    ))
}

fn decode_transcript(payload: &[u8], mut metadata: &[u8]) -> Result<SnapshotTranscript, BoxError> {
    let input = SnapshotTranscript::decode(payload, &mut metadata, WORLD)?;
    if !metadata.is_empty() {
        return Err("unexpected C16d transcript records".into());
    }
    Ok(input)
}

enum ExpectedAppearance {
    Reference(u32),
    Item(u32),
}

async fn assert_replay(
    input: SnapshotTranscript,
    expected: ExpectedAppearance,
) -> Result<(), BoxError> {
    let SnapshotTranscript {
        sequence,
        selected,
        refs,
        mut snapshots,
    } = input;
    let world = snapshots
        .iter()
        .position(|snapshot| snapshot.domain_id == WORLD)
        .ok_or("C16d world snapshot absent")?;
    let generation = if refs.is_empty() {
        // No item dictionary will be installed.
        CONTENT_GENERATION
    } else {
        decode_world_spatial_entities_snapshot_with_fields(
            &snapshots[world].payload,
            EntityFields::selected(&selected),
        )
        .map_err(SessionError::from)?
        .0
        .content_generation
    };
    let session = admit_snapshots(sequence, &selected, &snapshots, &refs, generation).await?;
    let own = session.world_entities().ok_or("C16d entities absent")?;
    let actor = own
        .iter()
        .find(|entity| &entity.entity.identity == own.own_identity())
        .ok_or("C16d own actor absent")?;
    match expected {
        ExpectedAppearance::Reference(expected) => assert!(matches!(
            actor.detail,
            EntityDetail::Actor { appearance_ref, .. } | EntityDetail::ActorWithIcons { appearance_ref, .. }
                if appearance_ref == expected
        )),
        ExpectedAppearance::Item(definition_ref) => {
            let expected = appearance::Look::Item { definition_ref };
            if !session.visible_appearances().iter().any(|visible| {
                visible.entity.entity == actor.entity && visible.look.as_ref() == Some(&expected)
            }) {
                return Err(format!(
                    "C16d replayed own look differs from expected item {definition_ref}"
                )
                .into());
            }
        }
    }

    let invalid = &mut snapshots[world].payload;
    invalid.truncate(invalid.len().saturating_sub(1));
    let refused = admit_snapshots(sequence, &selected, &snapshots, &refs, generation).await;
    assert!(matches!(
        refused
            .as_ref()
            .err()
            .and_then(|error| error.downcast_ref()),
        Some(SessionError::WorldSpatial(_))
    ));
    Ok(())
}
