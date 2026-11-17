use super::*;
use crate::analyser_transport::*;

#[test]
fn analyser_selected_facts_are_typed_and_invalid_batches_preserve_revision() -> Result<(), BoxError>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&10));
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[10],
                        &[10],
                        &[],
                        (1, spatial(0)),
                        &[(15, 1, 1, vec![])],
                    )
                    .await
                });
                let mut session = Session::admit(client, admission_with(&[10])?).await?;
                peer.await??;
                let facts = vec![
                    AnalyserFact::Kill { race: None },
                    AnalyserFact::Experience { raw: 20, gained: 0 },
                ];
                let payload = encode_analyser_facts(&facts).map_err(|e| format!("{e:?}"))?;
                let frame = encode_state_delta(1, 41, 15, 1, 2, 1, &payload)?;
                let event =
                    session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
                assert!(
                    matches!(event, SessionEvent::AnalyserFacts(delta) if delta.value == facts)
                );
                for (base, kind, payload) in [(1, 1, payload), (2, 2, vec![255]), (2, 1, vec![255])]
                {
                    let frame = encode_state_delta(1, 42, 15, base, base + 1, kind, &payload)?;
                    assert!(
                        session
                            .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
                            .is_err()
                    );
                    assert!(session.gated_revisions.contains(&(15, 2)));
                }
                session.selected_capabilities.clear();
                let frame = encode_state_delta(1, 43, 15, 2, 3, 1, &[])?;
                assert!(matches!(
                    session.apply_state_delta(43, decode_wire_envelope(&frame)?.payload()),
                    Err(SessionError::UnselectedDomain { domain_id: 15 })
                ));
                Ok(())
            })
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "analyser test thread")??
}

#[test]
fn c0486_selected_session_reports_the_attacker_of_a_damage_input_fact() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&486));
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[10, 486],
                        &[10, 486],
                        &[],
                        (1, spatial(0)),
                        &[(15, 1, 1, vec![])],
                    )
                    .await
                });
                let mut session = Session::admit(client, admission_with(&[10, 486])?).await?;
                peer.await??;
                let attacker = EntityRef {
                    identity: [7; 16],
                    generation: 3,
                };
                let hit = AnalyserFact::DamageInput {
                    value: NonZeroU32::new(9).ok_or("nonzero")?,
                    element: oteryn_protocol_oteryn::damage_element::DamageElement::Physical,
                    source: AnalyserDamageSource::Player,
                };
                let mut payload = Vec::new();
                for (fact, attacker) in [
                    (&hit, Some(&attacker)),
                    (&AnalyserFact::Kill { race: None }, None),
                ] {
                    let encoded = encode_attributed_analyser_fact(fact, attacker)
                        .map_err(|e| format!("{e:?}"))?;
                    payload.push(0x0a);
                    payload.push(u8::try_from(encoded.len())?);
                    payload.extend_from_slice(&encoded);
                }
                let frame = encode_state_delta(1, 41, 15, 1, 2, 1, &payload)?;
                let event =
                    session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
                let SessionEvent::AnalyserAttributedFacts(delta) = event else {
                    return Err("selected 486 must report attributed facts".into());
                };
                assert_eq!(
                    delta.value,
                    vec![
                        AttributedFact {
                            fact: hit,
                            attacker: Some(attacker)
                        },
                        AttributedFact {
                            fact: AnalyserFact::Kill { race: None },
                            attacker: None
                        },
                    ]
                );
                Ok(())
            })
        })
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "analyser test thread")??
}

#[test]
fn analyser_join_requires_an_empty_typed_snapshot() {
    assert!(crate::analyser_transport::validate_join(&[], &[]).is_ok());
    assert!(crate::analyser_transport::validate_join(&[10], &[]).is_err());
    for (snapshot_type, revision, payload, valid) in [
        (1, 1, vec![], true),
        (2, 1, vec![], false),
        (1, 0, vec![], false),
        (1, 1, vec![0], false),
    ] {
        assert_eq!(
            crate::analyser_transport::validate_join(
                &[10],
                &[GatedSnapshot {
                    domain_id: 15,
                    snapshot_type,
                    revision,
                    payload
                }]
            )
            .is_ok(),
            valid
        );
    }
}

#[test]
fn analyser_recovery_cannot_roll_back_confirmed_revision() {
    let mut row = GatedSnapshot {
        domain_id: 15,
        snapshot_type: 1,
        revision: 1,
        payload: vec![],
    };
    assert!(crate::analyser_transport::validate_snapshot(&row, Some(2)).is_err());
    row.revision = 3;
    assert!(crate::analyser_transport::validate_snapshot(&row, Some(2)).is_ok());
}
