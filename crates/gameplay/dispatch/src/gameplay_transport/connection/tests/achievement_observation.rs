// Exercise the live connection path, including its initial seed and existing idle cadence.
use super::*;
use oteryn_protocol_oteryn::achievement_notices::decode_achievement_earned;
use oteryn_protocol_oteryn::tutorial_notices::{
    CAPABILITY_TUTORIAL_NOTICES_V1, STATE_DOMAIN_TUTORIAL_NOTICES, TutorialNotice,
    decode_tutorial_notice,
};

const HELD: &str = "oteryn:achievement/allow_cookies";
const FISH: &str = "oteryn:achievement/here_fishy_fishy";
const SKIN: &str = "oteryn:achievement/skin_deep";

struct FactsAuthority {
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
    facts: RefCell<Vec<&'static str>>,
    reads: Cell<usize>,
    uses: Cell<usize>,
    granted: Cell<bool>,
    immediate: bool,
    periodic: bool,
    lose_control_after_use: bool,
}

impl FactsAuthority {
    fn new(immediate: bool) -> Self {
        Self {
            commands: Default::default(),
            facts: RefCell::new(vec![HELD]),
            reads: Cell::new(0),
            uses: Cell::new(0),
            granted: Cell::new(false),
            immediate,
            periodic: false,
            lose_control_after_use: false,
        }
    }

    fn grant(&self) {
        if !self.granted.replace(true) {
            self.facts.borrow_mut().extend([FISH, SKIN]);
        }
    }

    fn observation(&self) -> AchievementNoticeObservation {
        let facts = self.facts.borrow();
        let watermark = AchievementWatermark {
            fact_count: facts.len() as u32,
            total_points: facts.len() as u32 * 3,
        };
        AchievementNoticeObservation {
            watermark,
            facts: Some(
                facts
                    .iter()
                    .map(|key| AchievementEarned {
                        key: (*key).into(),
                        name: match *key {
                            HELD => "Allow Cookies?!",
                            FISH => "Here, Fishy Fishy!",
                            SKIN => "Skin-Deep",
                            _ => unreachable!(),
                        }
                        .into(),
                        watermark,
                    })
                    .collect(),
            ),
        }
    }
}

impl FreshAdmissionAuthority for FactsAuthority {
    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        Some(&self.commands)
    }

    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_achievement_notice_facts(
        &self,
        admitted: AdmittedSession,
    ) -> Option<AchievementNoticeObservation> {
        if admitted.controller?.account_id != NOTICE_ACCOUNT
            || (self.lose_control_after_use && self.uses.get() != 0)
        {
            return None;
        }
        let reads = self.reads.replace(self.reads.get() + 1);
        if self.periodic && reads != 0 {
            self.grant();
        }
        Some(self.observation())
    }

    async fn use_object(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        self.uses.set(self.uses.get() + 1);
        self.grant();
        UseOutcome {
            disposition: UseDisposition::Committed,
            committed: None,
            // Deliberately repeat the old immediate payload: the new-key gate must also
            // suppress a subsequent observation of the same receipt's fact.
            earned: if self.immediate {
                EarnedNotice::Earned(AchievementEarned {
                    key: FISH.into(),
                    name: "Here, Fishy Fishy!".into(),
                    watermark: self.observation().watermark,
                })
            } else {
                EarnedNotice::NoneEarned
            },
            gate_message_id: None,
            refusal_detail: None,
        }
    }
}

fn selected(caps: &[u32]) -> SessionContinuity {
    SessionContinuity {
        achievement_notice_revision: caps.contains(&8).then_some(0),
        selected_capabilities: SelectedCapabilities::select(
            &[
                OfferedCapability {
                    id: 8,
                    requires: &[],
                },
                OfferedCapability {
                    id: CAPABILITY_TUTORIAL_NOTICES_V1,
                    requires: &[],
                },
            ],
            caps,
        )
        .expect("notice capabilities"),
        ..SessionContinuity::FRESH
    }
}

fn outputs(frames: &[Vec<u8>]) -> Result<(Vec<AchievementEarned>, Vec<String>), Box<dyn Error>> {
    let mut achievements = Vec::new();
    let mut status = Vec::new();
    for frame in frames {
        let envelope = decode_wire_envelope(frame)?;
        if envelope.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
        match delta.domain_id {
            STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES => {
                achievements
                    .push(decode_achievement_earned(delta.payload).map_err(|e| format!("{e:?}"))?);
            }
            STATE_DOMAIN_TUTORIAL_NOTICES => {
                if let TutorialNotice::StatusText(text) =
                    decode_tutorial_notice(delta.payload).map_err(|e| format!("{e:?}"))?
                {
                    status.push(text);
                }
            }
            _ => {}
        }
    }
    Ok((achievements, status))
}

#[test]
fn p11_account_observation_deduplicates_immediate_grants_and_replay() -> Result<(), Box<dyn Error>>
{
    run(async {
        for immediate in [false, true] {
            let authority = FactsAuthority::new(immediate);
            let one = use_command(
                1,
                1,
                u64::from(COMMAND_TYPE_USE_INTENT),
                b"oteryn:placement/entry-chest",
                0,
            );
            let two = use_command(
                1,
                2,
                u64::from(COMMAND_TYPE_USE_INTENT),
                b"oteryn:placement/entry-chest",
                0,
            );
            let (end, frames) = drive_session(
                &authority,
                controlled(selected(&[8, 31]))?,
                &[one.clone(), one, two],
            )
            .await?;
            let results = frames
                .iter()
                .map(|frame| decode_wire_envelope(frame))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter(|envelope| envelope.message_type() == MessageType::CommandResult)
                .map(|envelope| {
                    oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map(|result| result.status)
                })
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(
                results,
                [
                    CommandStatus::Accepted,
                    CommandStatus::DuplicateReplay,
                    CommandStatus::Accepted
                ],
                "the retained command stream replays the original then executes the successor"
            );
            let (notices, texts) = outputs(&frames)?;
            assert_eq!(
                notices
                    .iter()
                    .map(|notice| notice.key.as_str())
                    .collect::<Vec<_>>(),
                [FISH, SKIN]
            );
            assert!(notices.iter().all(|notice| notice.watermark
                == AchievementWatermark {
                    fact_count: 3,
                    total_points: 9
                }));
            assert_eq!(
                texts,
                [
                    "Congratulations! You earned the achievement Here, Fishy Fishy!.",
                    "Congratulations! You earned the achievement Skin-Deep."
                ]
            );
            assert_eq!(
                authority.uses.get(),
                2,
                "a duplicate command never invokes its source twice"
            );
            let ended = ended_continuity(end)?;
            assert_eq!(ended.achievement_notice_revision, Some(2));
            assert_eq!(ended.tutorial_revision, 2);

            let resumed = SessionContinuity {
                connection_generation: 2,
                ..ended
            };
            let (_, frames) = drive_session(&authority, controlled(resumed)?, &[]).await?;
            assert_eq!(
                outputs(&frames)?,
                (Vec::new(), Vec::new()),
                "resync seeds every held key silently"
            );
        }
        Ok(())
    })
}

#[test]
fn p11_account_observation_respects_both_existing_notice_capabilities() -> Result<(), Box<dyn Error>>
{
    run(async {
        for caps in [&[][..], &[8][..], &[31][..]] {
            let authority = FactsAuthority::new(false);
            let command = use_command(
                1,
                1,
                u64::from(COMMAND_TYPE_USE_INTENT),
                b"oteryn:placement/entry-chest",
                0,
            );
            let (_, frames) =
                drive_session(&authority, controlled(selected(caps))?, &[command]).await?;
            let (notices, text) = outputs(&frames)?;
            assert_eq!(notices.len(), if caps.contains(&8) { 2 } else { 0 });
            assert_eq!(text.len(), if caps.contains(&31) { 2 } else { 0 });
            if caps.is_empty() {
                assert_eq!(
                    authority.reads.get(),
                    0,
                    "no unsolicited account observation"
                );
            }
        }
        Ok(())
    })
}

#[test]
fn p11_account_observation_runs_on_the_existing_liveness_cadence() -> Result<(), Box<dyn Error>> {
    run(async {
        tokio::time::pause();
        let authority = FactsAuthority {
            periodic: true,
            ..FactsAuthority::new(false)
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        let end = serve_admitted(
            &mut server,
            controlled(selected(&[8, 31]))?,
            &authority,
            IDLE_LIVENESS,
        )
        .await;
        assert!(matches!(end, ConnectionEnd::AdmittedThenControlLost(_)));
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let (notices, text) = outputs(&split_frames(&output)?)?;
        assert_eq!(
            notices
                .iter()
                .map(|notice| notice.key.as_str())
                .collect::<Vec<_>>(),
            [FISH, SKIN]
        );
        assert_eq!(text.len(), 2);
        assert_eq!(
            authority.uses.get(),
            0,
            "passive grants need no client action"
        );
        assert!(
            authority.reads.get() > 2,
            "repeated observations do not repeat notices"
        );
        Ok(())
    })
}

#[test]
fn p11_account_observation_fails_closed_after_control_changes() -> Result<(), Box<dyn Error>> {
    run(async {
        for immediate in [false, true] {
            let authority = FactsAuthority {
                lose_control_after_use: true,
                ..FactsAuthority::new(immediate)
            };
            let command = use_command(
                1,
                1,
                u64::from(COMMAND_TYPE_USE_INTENT),
                b"oteryn:placement/entry-chest",
                0,
            );
            let (end, frames) =
                drive_session(&authority, controlled(selected(&[8, 31]))?, &[command]).await?;
            assert_eq!(outputs(&frames)?, (Vec::new(), Vec::new()));
            assert_eq!(ended_continuity(end)?.achievement_notice_revision, Some(0));
        }
        Ok(())
    })
}
