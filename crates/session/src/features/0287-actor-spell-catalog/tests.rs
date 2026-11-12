use super::*;
use oteryn_protocol_oteryn::actor_spell_catalog::*;

#[test]
fn gap_spell_catalog_session_replacement_and_preservation() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[287],
                        &[287],
                        &[],
                        (1, spatial(0)),
                        &[(100, 1, 1, vec![])],
                    )
                    .await
                });
                let mut session = Session::admit(client, admission_with(&[287])?).await?;
                peer.await??;
                assert!(
                    session.gated_snapshot(100).is_some(),
                    "catalogue snapshot must be retained"
                );
                let entry = SpellCatalogEntry {
                    cast_index: 1,
                    name: "Light Healing".into(),
                    words: "exura".into(),
                    level: 8,
                    mana: 20,
                    mana_percent: false,
                    soul: 0,
                    learned: false,
                    available: false,
                    cooldown_groups: vec![1],
                    rune_item: None,
                };
                let mut learned = entry.clone();
                learned.learned = true;
                learned.available = true;
                for (base, entries) in [(1, vec![entry]), (2, vec![learned])] {
                    let bytes = encode_catalog(&entries).map_err(|e| format!("{e:?}"))?;
                    let frame = encode_state_delta(1, 41, 100, base, base + 1, 1, &bytes)?;
                    session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
                    let row = session.gated_snapshot(100).ok_or("catalogue")?;
                    assert_eq!(row.revision, base + 1);
                    assert_eq!(
                        decode_catalog(&row.payload).map_err(|e| format!("{e:?}"))?,
                        entries
                    );
                }
                let confirmed = session.gated_snapshot(100).ok_or("catalogue")?.clone();
                for (base, new, kind, bytes) in [
                    (2, 3, 1, vec![]),
                    (3, 5, 1, vec![]),
                    (3, 4, 2, vec![]),
                    (3, 4, 1, vec![255]),
                ] {
                    let frame = encode_state_delta(1, 42, 100, base, new, kind, &bytes)?;
                    assert!(
                        session
                            .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
                            .is_err()
                    );
                    let row = session.gated_snapshot(100).ok_or("catalogue")?;
                    assert_eq!(row, &confirmed);
                }
                assert!(
                    session
                        .record_spell_catalog(3, 4, 1, &vec![0; MAX_CATALOG_BYTES + 1])
                        .is_err()
                );
                assert_eq!(session.gated_snapshot(100), Some(&confirmed));
                let frame = encode_state_delta(1, 43, 100, 3, 4, 1, &[])?;
                session.apply_state_delta(43, decode_wire_envelope(&frame)?.payload())?;
                assert!(
                    session
                        .spell_catalog()?
                        .ok_or("catalogue")?
                        .entries
                        .is_empty()
                );
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[287],
                        &[287],
                        &[],
                        (1, spatial(0)),
                        &[(100, 1, 1, vec![])],
                    )
                    .await?;
                    write_frame(&mut server, &encode_state_delta(1, 41, 100, 1, 2, 1, &[])?)
                        .await?;
                    write_frame(
                        &mut server,
                        &encode_state_delta(1, 42, 100, 2, 3, 1, &[255])?,
                    )
                    .await?;
                    Ok::<_, BoxError>(())
                });
                let mut fresh = Session::admit(client, admission_with(&[287])?).await?;
                assert_eq!(fresh.spell_catalog()?.ok_or("fresh catalogue")?.revision, 1);
                fresh.read_gated_delta(100, 1).await?;
                assert_eq!(fresh.spell_catalog()?.ok_or("fresh catalogue")?.revision, 2);
                assert!(fresh.read_gated_delta(100, 1).await.is_err());
                assert_eq!(fresh.spell_catalog()?.ok_or("fresh catalogue")?.revision, 2);
                peer.await??;
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move { join_peer(&mut server, &[], &[], &[]).await });
                let mut fresh = Session::admit(client, admission_with(&[])?).await?;
                peer.await??;
                assert!(fresh.gated_snapshot(100).is_none());
                let frame = encode_state_delta(1, 41, 100, 0, 1, 1, &[])?;
                assert!(
                    fresh
                        .apply_state_delta(41, decode_wire_envelope(&frame)?.payload())
                        .is_err()
                );
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "catalogue thread")?
}

#[test]
fn gap_spell_catalog_rejects_invalid_admission() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for rows in [
                    vec![],
                    vec![(100, 0, 1, vec![])],
                    vec![(100, 2, 1, vec![])],
                    vec![(100, 1, 2, vec![])],
                    vec![(100, 1, 1, vec![255])],
                ] {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(async move {
                        join_peer_with(&mut server, &[287], &[287], &[], (1, spatial(0)), &rows)
                            .await
                    });
                    assert!(
                        Session::admit(client, admission_with(&[287])?)
                            .await
                            .is_err()
                    );
                    peer.await??;
                }
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "catalogue thread")?
}
