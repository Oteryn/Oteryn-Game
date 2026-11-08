//! W22a client replay: the rat corpse the Character opened (domain 11) goes through
//! `Session::admit` and `Session::apply_state_delta` (`domain_dispatch.rs`) and is read through
//! `Session::open_container`. The views are session-owned, so the row re-encodes them with the
//! production `item_view::encode_open_container` (the C17c practice). Moving the first rolled stack
//! removes it from the corpse; a malformed delta is refused and leaves the corpse unchanged.
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
        handle: ItemHandle::new(handle).ok_or("handle")?,
        item_definition_ref: std::num::NonZeroU32::new(reference).ok_or("reference")?,
        count: std::num::NonZeroU32::new(count).ok_or("count")?,
        sub_type: 0,
    })
}

fn fixture() -> Result<Capture, BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
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
    Ok(Capture {
        before: encode(vec![entry(51, 3607, 1)?, entry(52, 3031, 3)?])?,
        after: encode(vec![entry(52, 3031, 3)?])?,
        expected: "before=3607:1;3031:3\nafter=3031:3\ntable=3031:1:4;3607:1:1\n".into(),
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
        let (before_payload, after_payload) = (capture.before.clone(), capture.after.clone());
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                SELECTED,
                SELECTED,
                &[],
                item_snapshot(vec![])?,
                &[
                    inventory_domain(3, &CharacterInventory::default())?,
                    (
                        item_view::STATE_DOMAIN_OPEN_CONTAINER,
                        3,
                        item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1,
                        before_payload,
                    ),
                ],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    41,
                    item_view::STATE_DOMAIN_OPEN_CONTAINER,
                    3,
                    4,
                    item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
                    &after_payload,
                )?,
            )
            .await?;
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(SELECTED)?).await?;
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
        session.service_liveness(Duration::from_millis(100)).await?;
        assert_eq!(listed(session.open_container()), value(expected, "after")?);
        assert_ne!(value(expected, "before")?, value(expected, "after")?);
        let applied = session.open_container().cloned();
        let bad = encode_state_delta(
            1,
            42,
            item_view::STATE_DOMAIN_OPEN_CONTAINER,
            4,
            5,
            item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(42, decode_wire_envelope(&bad)?.payload())
                .is_err()
        );
        assert_eq!(session.open_container().cloned(), applied);
        drop(session);
        peer.abort();
        Ok(())
    })?
}
