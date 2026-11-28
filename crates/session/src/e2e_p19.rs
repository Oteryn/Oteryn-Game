//! P19 client forge-marker replay.
//!
//! p19-appearance-snapshot.bin is the unmodified appearance snapshot payload.
//! p19-appearance-deltas.bin uses the OTR1 little-endian transcript format
//! documented in e2e_c16c.rs, with the appearance snapshot as the external
//! entry. The world snapshot and world deltas are included to reconstruct
//! visibility and appearance references, in order with the appearance deltas.
//! p19-expected.txt records the 32 hex identity digits, decimal generation and
//! decimal forge_stacks, separated by spaces, followed by a newline.

use super::{appearance_session_tests::appearance_fixture, replay_input::*, *};
use crate::appearance_state::AppearanceUpdate;
use oteryn_protocol_oteryn::appearance;
use replay_capture::P19_FILES;

#[test]
fn e2e_p19_forge_marker_replay() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| block_on(replay_forge())?)?
        .join()
        .map_err(|_| "P19 replay thread")?
}

async fn replay_forge() -> Result<(), BoxError> {
    let fixture = forge_fixture()?;
    let mut delayed = forge_fixture()?;
    let reframe = |frame: &[u8], sequence| -> Result<Vec<u8>, BoxError> {
        let envelope = decode_wire_envelope(frame)?;
        let delta = decode_state_delta(envelope.payload())?;
        Ok(encode_state_delta(
            1,
            sequence,
            delta.domain_id,
            delta.base_revision,
            delta.new_revision,
            delta.delta_type,
            delta.payload,
        )?)
    };
    delayed.deltas = vec![
        reframe(&fixture.deltas[1], 41)?,
        reframe(&fixture.deltas[0], 43)?,
    ];
    let rejected = assert_forge(&delayed, &mut delayed.admit().await?)
        .expect_err("a marker published after spatial entry must fail replay");
    assert_eq!(
        rejected.to_string(),
        "P19 spatial entry preceded its exact marker"
    );

    let mut forgotten = forge_fixture()?;
    let (appearance, _, _) = appearance_fixture::snapshot()?;
    let forget = appearance::Mappings {
        context: appearance.catalogue.context,
        upserts: vec![],
        forgets: vec![fixture.entity],
    }
    .encode()
    .map_err(|error| format!("{error:?}"))?;
    forgotten
        .deltas
        .insert(1, encode_state_delta(1, 42, APPEARANCE, 2, 3, 1, &forget)?);
    let rejected = assert_forge(&forgotten, &mut forgotten.admit().await?)
        .expect_err("a marker withdrawn before spatial entry must fail replay");
    assert_eq!(
        rejected.to_string(),
        "P19 spatial entry preceded its exact marker"
    );

    let mut session = fixture.admit().await?;
    let corrupt = encode_state_delta(1, 41, APPEARANCE, 1, 2, 1, &[0])?;
    assert!(matches!(
        apply_frame(&mut session, &corrupt),
        Err(SessionError::Protocol(
            FoundationProtocolError::MalformedEnvelope
        ))
    ));
    assert!(session.raw_appearance_deltas().is_empty());
    assert_eq!(session.visible_appearances().len(), 1);

    assert_forge(&fixture, &mut session)?;
    let raw = session.raw_appearance_deltas();
    assert_eq!(raw.len(), 1);
    let envelope = decode_wire_envelope(&fixture.deltas[0])?;
    let original = decode_state_delta(envelope.payload())?;
    assert_eq!(raw[0].payload.as_ref(), original.payload);

    let files = session
        .p19_capture_files(fixture.entity)
        .ok_or("P19 capture missing")?;
    assert_eq!(files[0].1, fixture.snapshots[1].payload);
    let input = round_trip(files, P19_FILES, APPEARANCE)?;
    assert_eq!(input.deltas, fixture.deltas);
    assert_forge(&input, &mut input.admit().await?)?;

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        for (directory, fiendish) in [(path.to_path_buf(), false), (path.join("fiendish"), true)] {
            let input = ReplayInput::read(&directory, P19_FILES, APPEARANCE)?;
            assert!(if fiendish {
                input.expected == 15
            } else {
                (1..=5).contains(&input.expected)
            });
            assert_forge(&input, &mut input.admit().await?)?;
        }
    }
    Ok(())
}

fn assert_forge(input: &ReplayInput, session: &mut Session<DuplexStream>) -> Result<(), BoxError> {
    let mut stacks = None;
    let mut entered = false;
    for event in input.apply(session)? {
        match event {
            SessionEvent::Appearance(delta) => {
                if let AppearanceUpdate::Mappings(mappings) = delta.value {
                    if mappings.forgets.contains(&input.entity) {
                        stacks = None;
                    }
                    if let Some(binding) = mappings
                        .upserts
                        .iter()
                        .find(|entry| entry.entity == input.entity)
                    {
                        stacks = binding.forge_stacks;
                        assert_eq!(stacks, Some(input.expected));
                    }
                }
            }
            SessionEvent::WorldSpatialEntities(delta)
                if delta
                    .value
                    .enter
                    .iter()
                    .any(|entry| entry.entity == input.entity) =>
            {
                if stacks != Some(input.expected) {
                    return Err("P19 spatial entry preceded its exact marker".into());
                }
                entered = true;
            }
            _ => {}
        }
    }
    assert!(
        entered,
        "P19 exact creature never entered spatial visibility"
    );
    let entry = session
        .visible_appearances()
        .into_iter()
        .find(|entry| entry.entity.entity == input.entity)
        .ok_or("P19 creature not visible after replay")?;
    assert_eq!(entry.entity.kind, EntityKind::Creature);
    assert_eq!(entry.forge_stacks, Some(input.expected));
    Ok(())
}

fn forge_fixture() -> Result<ReplayInput, BoxError> {
    let (appearance, _, (_, spatial)) = appearance_fixture::snapshot()?;
    let mut monster = actor(EntityKind::Creature, 0xb0, 1, at(1, 0));
    monster.detail = EntityDetail::Actor {
        direction: StepDirection::South,
        appearance_ref: 2,
        health_percent: 100,
    };
    let mut binding = appearance.bindings[0].clone();
    binding.entity = monster.entity;
    binding.appearance_ref = 2;
    let stacks = 15;
    binding.forge_stacks = Some(stacks);
    let mappings = appearance::Mappings {
        context: appearance.catalogue.context,
        upserts: vec![binding],
        forgets: vec![],
    }
    .encode()
    .map_err(|e| format!("{e:?}"))?;
    let enter = entities_delta(at(0, 0), vec![monster], vec![], vec![]);
    let enter = encode_world_spatial_entities_delta(&enter).map_err(|e| format!("{e:?}"))?;
    Ok(ReplayInput {
        sequence: 40,
        selected: vec![6, 25, 104],
        refs: vec![],
        snapshots: vec![
            GatedSnapshot {
                domain_id: WORLD,
                revision: 5,
                snapshot_type: 2,
                payload: spatial,
            },
            GatedSnapshot {
                domain_id: APPEARANCE,
                revision: 1,
                snapshot_type: 1,
                payload: appearance.encode().map_err(|e| format!("{e:?}"))?,
            },
        ],
        deltas: vec![
            encode_state_delta(1, 41, APPEARANCE, 1, 2, 1, &mappings)?,
            encode_state_delta(1, 43, WORLD, 5, 6, 2, &enter)?,
        ],
        entity: monster.entity,
        expected: stacks,
    })
}

#[test]
fn e2e_replay_history_is_bounded_and_rejects_incomplete_captures() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| block_on(bounded_history())?)?
        .join()
        .map_err(|_| "bounded replay thread")?
}

async fn bounded_history() -> Result<(), BoxError> {
    let fixture = forge_fixture()?;
    let mut session = fixture.admit().await?;
    assert_forge(&fixture, &mut session)?;
    let delta = entities_delta(at(0, 0), vec![], vec![own(at(0, 0))], vec![]);
    let spatial = encode_world_spatial_entities_delta(&delta).map_err(|e| format!("{e:?}"))?;
    let (snapshot, _, _) = appearance_fixture::snapshot()?;
    let mut catalogue = snapshot.catalogue;
    for offset in 0..u64::try_from(REPLAY_DELTA_LIMIT)? {
        catalogue.root_revision += 1;
        let payload = catalogue.encode().map_err(|e| format!("{e:?}"))?;
        let sequence = 44 + 2 * offset;
        apply_frame(
            &mut session,
            &encode_state_delta(1, sequence, APPEARANCE, 2 + offset, 3 + offset, 2, &payload)?,
        )?;
        apply_frame(
            &mut session,
            &encode_state_delta(1, sequence + 1, WORLD, 6 + offset, 7 + offset, 2, &spatial)?,
        )?;
    }
    let world = session.raw_world_entity_deltas();
    let appearance = session.raw_appearance_deltas();
    assert_eq!(world.len(), REPLAY_DELTA_LIMIT);
    assert_eq!(appearance.len(), REPLAY_DELTA_LIMIT);
    assert_eq!(world[0].base_revision, 6);
    assert_eq!(appearance[0].base_revision, 2);
    assert!(session.p19_capture_files(fixture.entity).is_none());

    // Replacing only appearance preserves the world history; a fresh scope
    // builds both histories from its own snapshot instead of inheriting either.
    let old = snapshot_state::SnapshotState::capture(&session);
    let snapshots = fixture
        .snapshots
        .iter()
        .map(|snapshot| DomainSnapshot {
            domain_id: snapshot.domain_id,
            revision: snapshot.revision,
            snapshot_type: snapshot.snapshot_type,
            payload: &snapshot.payload,
        })
        .collect::<Vec<_>>();
    for (domains, previous) in [(&snapshots[1..], Some(&old)), (snapshots.as_slice(), None)] {
        let [_, chunk, _] = encode_single_chunk_snapshot(1, 2, 120, domains)?;
        let envelope = decode_wire_envelope(&chunk)?;
        let (_, body) = oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())?;
        let next =
            snapshot_state::SnapshotState::decode(body, 120, &fixture.selected, None, previous)?;
        next.install(&mut session);
        assert!(session.raw_appearance_deltas().is_empty());
        assert_eq!(
            session.raw_world_entity_deltas().len(),
            previous.map_or(0, |_| REPLAY_DELTA_LIMIT)
        );
        assert_eq!(
            session
                .gated_snapshot(APPEARANCE)
                .ok_or("replacement appearance")?
                .payload,
            fixture.snapshots[1].payload
        );
    }
    Ok(())
}
