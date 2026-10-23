// Transport proof reuses the existing StepAuthority and connection driver. Reducer and
// PostgreSQL authority tests separately exercise the production step and progress owners.
#[test]
fn tutorial_notices_follow_the_hole_and_the_forcefield_steps_through_dispatch()
-> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::tutorial_notices::{
        STATE_DOMAIN_TUTORIAL_NOTICES, TutorialNotice, decode_tutorial_notice,
    };
    run(async {
        for enabled in [false, true] {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
            };
            let admitted = paced_session(if enabled { &[31] } else { &[] })?;
            let (end, frames) = drive_session(
                &authority,
                admitted,
                &[
                    command(ADMITTED_GENERATION, 1, 1, StepDirection::North),
                    command(ADMITTED_GENERATION, 2, 1, StepDirection::East),
                ],
            )
            .await?;
            let mut notices = Vec::new();
            let mut preceding_command = 0;
            for frame in frames {
                let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    preceding_command += 1;
                } else if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                    if delta.domain_id == STATE_DOMAIN_TUTORIAL_NOTICES {
                        assert_eq!(delta.base_revision, notices.len() as u64);
                        assert_eq!(delta.new_revision, delta.base_revision + 1);
                        notices.push((
                            preceding_command,
                            decode_tutorial_notice(delta.payload)
                                .map_err(|error| format!("{error:?}"))?,
                        ));
                    }
                }
            }
            if enabled {
                assert_eq!(
                    notices,
                    vec![
                        (
                            1,
                            TutorialNotice::StatusText(
                                oteryn_gameplay_quests::tutorial::HOLE_TEXT.into()
                            )
                        ),
                        (2, TutorialNotice::Hint(2)),
                    ]
                );
            } else {
                assert!(notices.is_empty());
            }
            let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
                return Err("unexpected end".into());
            };
            assert_eq!(
                admitted.continuity.tutorial_revision,
                if enabled { 2 } else { 0 }
            );
        }
        Ok(())
    })
}
