//! Replays the two original open-container delta payloads captured by W22a.client.
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::{encode_state_delta, item_view};

const SELECTED: &[u32] = &[4, 6, 12];

struct Capture {
    before: Vec<u8>,
    after: Vec<u8>,
    expected: String,
}

fn entry(handle: u64, reference: u32, count: u32) -> Result<ItemEntry, BoxError> {
    Ok(ItemEntry {
        appearance_id: 0,
        handle: ItemHandle::new(handle).ok_or("handle")?,
        item_definition_ref: std::num::NonZeroU32::new(reference).ok_or("reference")?,
        count: std::num::NonZeroU32::new(count).ok_or("count")?,
        sub_type: 0,
    })
}

fn fixture() -> Result<Capture, BoxError> {
    if let Some(path) = std::env::var_os("W22A_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok(Capture {
            before: std::fs::read(path.join("w22a-before.bin"))?,
            after: std::fs::read(path.join("w22a-after.bin"))?,
            expected: std::fs::read_to_string(path.join("w22a-expected.txt"))?,
        });
    }
    let encode = |entries: Vec<ItemEntry>| {
        item_view::encode_open_container(&OpenContainer {
            container_handle: ItemHandle::new(50),
            entries,
        })
        .map_err(|error| BoxError::from(format!("{error:?}")))
    };
    let delta = |sequence, base, next, entries| -> Result<Vec<u8>, BoxError> {
        let frame = encode_state_delta(
            1,
            sequence,
            item_view::STATE_DOMAIN_OPEN_CONTAINER,
            base,
            next,
            item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
            &encode(entries)?,
        )?;
        Ok(decode_wire_envelope(&frame)?.payload().to_vec())
    };
    Ok(Capture {
        before: delta(41, 3, 4, vec![entry(51, 3607, 1)?, entry(52, 3031, 3)?])?,
        after: delta(43, 4, 5, vec![entry(52, 3031, 3)?])?,
        expected: "session=unit\nopen_sequence=41\npickup_sequence=43\nappearances=false\nexpanded=false\nbefore=3607:1;3031:3\nafter=3031:3\ntable=3031:1:4;3607:1:1\n".into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w22a expected lacks {key}").into())
}

fn listed(view: Option<&OpenContainer>) -> String {
    view.map(|view| {
        view.entries
            .iter()
            .map(|entry| format!("{}:{}", entry.item_definition_ref, entry.count))
            .collect::<Vec<_>>()
            .join(";")
    })
    .unwrap_or_default()
}

#[test]
fn e2e_w22a_rat_corpse_loot_replay() -> Result<(), BoxError> {
    block_on(async {
        let capture = fixture()?;
        let expected = capture.expected.as_str();
        let open_sequence = value(expected, "open_sequence")?.parse::<u64>()?;
        let pickup_sequence = value(expected, "pickup_sequence")?.parse::<u64>()?;
        assert!(open_sequence != 0 && pickup_sequence > open_sequence);
        let base = oteryn_protocol_oteryn::decode_state_delta(&capture.before)?.base_revision;
        let mut selected = SELECTED.to_vec();
        if value(expected, "appearances")?.parse::<bool>()? {
            selected.push(item_view::CAPABILITY_ITEM_APPEARANCE_V1);
        }
        if value(expected, "expanded")?.parse::<bool>()? {
            selected.push(item_view::CAPABILITY_CORPSE_CONTAINER_48_V1);
        }

        // Initialize the empty domain. Only the two captured payloads drive the proof.
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let server_selected = selected.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &server_selected,
                &server_selected,
                &[],
                item_snapshot(vec![])?,
                &[
                    inventory_domain(3, &CharacterInventory::default())?,
                    (
                        item_view::STATE_DOMAIN_OPEN_CONTAINER,
                        base,
                        item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1,
                        item_view::encode_open_container(&OpenContainer::default())
                            .map_err(|error| format!("{error:?}"))?,
                    ),
                ],
            )
            .await
        });
        let admission = Admission {
            supported_capabilities: &selected,
            ..admission_with(SELECTED)?
        };
        let mut session = Session::admit(client, admission).await?;
        peer.await??;
        assert_eq!(session.open_container(), Some(&OpenContainer::default()));
        assert!(session.take_open_container_delta_capture().is_empty());
        session.begin_open_container_delta_capture();
        let SessionEvent::OpenContainer(opened) =
            session.apply_state_delta(open_sequence, &capture.before)?
        else {
            return Err("open payload is not an open-container delta".into());
        };
        assert_eq!(opened.server_sequence, open_sequence);
        assert_eq!(listed(session.open_container()), value(expected, "before")?);
        // Every rolled stack lies inside the server's rat table range.
        let table: Vec<(u32, u32, u32)> = value(expected, "table")?
            .split(';')
            .map(|row| {
                let mut parts = row.split(':').map(str::parse::<u32>);
                Ok::<_, BoxError>((
                    parts.next().ok_or("ref")??,
                    parts.next().ok_or("min")??,
                    parts.next().ok_or("max")??,
                ))
            })
            .collect::<Result<_, _>>()?;
        for entry in &session.open_container().ok_or("corpse absent")?.entries {
            let (_, min, max) = table
                .iter()
                .find(|(reference, ..)| *reference == entry.item_definition_ref.get())
                .ok_or("rolled definition outside the rat table")?;
            assert!((*min..=*max).contains(&entry.count.get()));
        }
        let mut remaining = opened.value;
        assert!(remaining.container_handle.is_some());
        assert!(!remaining.entries.is_empty());
        remaining.entries.remove(0);
        let SessionEvent::OpenContainer(picked_up) =
            session.apply_state_delta(pickup_sequence, &capture.after)?
        else {
            return Err("pickup payload is not an open-container delta".into());
        };
        assert_eq!(picked_up.server_sequence, pickup_sequence);
        assert_eq!(picked_up.value, remaining);
        assert_eq!(listed(session.open_container()), value(expected, "after")?);
        assert_ne!(value(expected, "before")?, value(expected, "after")?);
        let applied = session.open_container().cloned();
        let bad_sequence = pickup_sequence
            .checked_add(1)
            .ok_or("capture sequence overflow")?;
        let new_revision = picked_up
            .new_revision
            .checked_add(1)
            .ok_or("capture revision overflow")?;
        let bad = encode_state_delta(
            1,
            bad_sequence,
            item_view::STATE_DOMAIN_OPEN_CONTAINER,
            picked_up.new_revision,
            new_revision,
            item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(bad_sequence, decode_wire_envelope(&bad)?.payload())
                .is_err()
        );
        assert_eq!(session.open_container().cloned(), applied);
        let captured = session.take_open_container_delta_capture();
        let [recorded_open, recorded_pickup] = captured.as_slice() else {
            return Err("capture did not retain exactly the two successful deltas".into());
        };
        assert_eq!(recorded_open.server_sequence, open_sequence);
        assert_eq!(recorded_open.payload, capture.before);
        assert_eq!(recorded_pickup.server_sequence, pickup_sequence);
        assert_eq!(recorded_pickup.payload, capture.after);
        assert!(session.take_open_container_delta_capture().is_empty());
        println!(
            "W22A_REPLAY session={} open_sequence={open_sequence} pickup_sequence={pickup_sequence}",
            value(expected, "session")?,
        );
        Ok(())
    })?
}
