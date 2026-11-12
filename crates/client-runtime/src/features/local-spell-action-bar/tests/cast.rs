//! Exercise CombatCommand::Cast over the real admitted Session and command exchange.
use crate::combat::{CombatCommand, CombatReport, run_combat};
use crate::spell::SpellFeedback;
use oteryn_session::{
    Admission, CharacterId, Session, SpellCastDisposition, read_frame, write_frame,
};
use std::{num::NonZeroU32, time::Duration};
use tokio::io::DuplexStream;
type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[test]
fn normal_cast_preserves_held_target_with_aim_capability() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    for selected in [true, false] {
                        for held in [true, false] {
                            let (client, mut server) = tokio::io::duplex(65536);
                            let peer = tokio::spawn(async move {
                                admit_peer(&mut server, selected, held).await?;
                                // Flame, Energy and Ice Strike all take the same normal-cast route.
                                for index in 1..=3 {
                                    let mut intent = vec![8, index, 16, if held { 2 } else { 1 }];
                                    if held {
                                        intent.extend_from_slice(&[32, 1]);
                                    }
                                    let mut command = vec![8, 6 + index, 16, 3];
                                    bytes(&mut command, 4, &intent);
                                    assert_eq!(
                                        read_frame(&mut server).await?,
                                        envelope(7, 1, 0, &command)
                                    );
                                    let mut result = vec![8, 6 + index, 16, 1];
                                    // CoolingDown is a non-Rejected result, so probing cannot retry.
                                    bytes(&mut result, 5, &[8, 2]);
                                    write_frame(
                                        &mut server,
                                        &envelope(8, 1, 40 + u64::from(index), &result),
                                    )
                                    .await?;
                                }
                                Ok::<(), BoxError>(())
                            });
                            let mut session = Session::admit(
                                client,
                                Admission {
                                    appearance_items: None,
                                    schema_revision: 1,
                                    character_id: CharacterId::decode(&identity(4))?,
                                    admission_material: b"grant",
                                    client_build_id: "normal-cast-test",
                                    supported_capabilities: &[6, 17, 480],
                                    deadline: Duration::from_secs(5),
                                },
                            )
                            .await?;
                            assert_eq!(
                                session.combat_state().is_some_and(|s| s.target.is_some()),
                                held
                            );
                            let mut feedback = SpellFeedback::default();
                            for index in 1..=3 {
                                let report = run_combat(
                                    &mut session,
                                    &mut feedback,
                                    CombatCommand::Cast(
                                        NonZeroU32::new(index).ok_or("spell index")?,
                                    ),
                                )
                                .await?;
                                assert_eq!(
                                    report,
                                    CombatReport::Cast(SpellCastDisposition::CoolingDown)
                                );
                            }
                            peer.await??;
                        }
                    }
                    Ok::<(), BoxError>(())
                })
        })?
        .join()
        .map_err(|_| "cast test thread panicked")?
}

async fn admit_peer(stream: &mut DuplexStream, selected: bool, held: bool) -> Result<(), BoxError> {
    read_frame(stream).await?;
    let mut accepted = Vec::new();
    for marker in 1..=3 {
        bytes(&mut accepted, marker, &identity(marker as u8));
    }
    accepted.extend_from_slice(&[32, 1, 48, 7, 56, 1, 64, 1, 72, 1]);
    for capability in [6, 17].into_iter().chain(selected.then_some(480)) {
        scalar(&mut accepted, 10, capability);
    }
    write_frame(stream, &envelope(2, 0, 0, &accepted)).await?;
    let mut spatial = Vec::new();
    bytes(&mut spatial, 1, &[0x11; 32]);
    bytes(&mut spatial, 2, &[]);
    bytes(&mut spatial, 3, &[1; 16]);
    let mut own = vec![8, 1];
    bytes(&mut own, 2, &[1; 16]);
    own.extend_from_slice(&[24, 1]);
    bytes(&mut own, 4, &[]);
    own.extend_from_slice(&[40, 1, 48, 1, 56, 100]);
    bytes(&mut spatial, 4, &own);
    let mut combat = Vec::new();
    if held {
        let mut target = Vec::new();
        bytes(&mut target, 1, &[7; 16]);
        scalar(&mut target, 2, 3);
        bytes(&mut combat, 1, &target);
    }
    combat.extend_from_slice(&[16, 1, 24, 1, 32, 1]);
    let mut body = Vec::new();
    for (domain, payload) in [
        (1, spatial.as_slice()),
        (2, &[][..]),
        (10, combat.as_slice()),
    ] {
        let mut entry = vec![8, domain, 16, 1, 24, if domain == 1 { 2 } else { 1 }];
        bytes(&mut entry, 4, payload);
        bytes(&mut body, 1, &entry);
    }
    let mut begin = vec![8, 1, 16, 1];
    scalar(&mut begin, 3, body.len() as u64);
    scalar(&mut begin, 4, 40);
    let mut chunk = vec![8, 1];
    bytes(&mut chunk, 3, &body);
    for frame in [
        envelope(11, 1, 0, &begin),
        envelope(12, 1, 0, &chunk),
        envelope(13, 1, 0, &[8, 1]),
    ] {
        write_frame(stream, &frame).await?;
    }
    Ok(())
}

fn identity(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

fn envelope(kind: u64, generation: u64, sequence: u64, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    scalar(&mut frame, 1, kind);
    scalar(&mut frame, 2, generation);
    scalar(&mut frame, 3, sequence);
    bytes(&mut frame, 4, payload);
    frame
}

fn scalar(output: &mut Vec<u8>, field: u32, value: u64) {
    if value != 0 {
        varint(output, u64::from(field) << 3);
        varint(output, value);
    }
}

fn bytes(output: &mut Vec<u8>, field: u32, value: &[u8]) {
    varint(output, (u64::from(field) << 3) | 2);
    varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}
