use crate::{
    combat_input::{CombatCommand, CombatReport, PlayCommand},
    play::{PlayEvent, PlayLink, Stepper, play_channel, run_session},
};
use oteryn_client_runtime::{appearance, raw_feature::FeatureCommand};
use oteryn_session::{
    ActorPosition, AppliedDelta, CommandStatus, EntityDetail, EntityKind, EntityRef,
    GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
    WorldSpatialEntitiesDelta, WorldSpatialEntity, actor_motion::ActorMotion,
};
use std::{cell::Cell, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    DelayedMount,
    NoMountOutfit,
    DeltaBeforeReply,
}

struct MountPeer<'a> {
    link: &'a PlayLink,
    scenario: Scenario,
    events: Vec<SessionEvent>,
    mount_look_type: u32,
    received: Vec<u8>,
    toggled_during_publish: Cell<bool>,
    idle_turn: u8,
}

impl MountPeer<'_> {
    fn sample_motion(&mut self, mount_look_type: u32) {
        self.mount_look_type = mount_look_type;
        let actor = self.actor();
        self.events
            .push(SessionEvent::WorldSpatialEntities(AppliedDelta {
                server_sequence: 1,
                base_revision: 1,
                new_revision: 2,
                value: WorldSpatialEntitiesDelta {
                    content_generation: [0; 32],
                    actor_position: actor.position,
                    enter: Vec::new(),
                    update: vec![actor],
                    leave: Vec::new(),
                },
            }));
    }

    fn request_no_mount_outfit(&self) -> Result<(), SessionError> {
        let change = appearance::Change {
            context: appearance::Context {
                content_generation: [1; 32],
                cosmetics_digest: [2; 32],
            },
            expected_root_revision: 1,
            prior: None,
            selection: appearance::Selection {
                outfit_key: "citizen".into(),
                colours: [0; 4],
                addons: 0,
                mount_key: None,
            },
            content_revision: "test".into(),
        };
        self.link
            .commands
            .send(PlayCommand::Feature(FeatureCommand {
                capability: 25,
                kind: 25,
                payload: change.encode().map_err(|_| SessionError::SessionUnusable)?,
            }))
            .map_err(|_| SessionError::SessionUnusable)
    }
}

impl Stepper for MountPeer<'_> {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 25 | 502)
    }

    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        if (command.capability, command.kind) == (25, 25) {
            assert!(self.scenario == Scenario::NoMountOutfit);
            assert_eq!(self.received, [1]);
            let change = appearance::Change::decode(&command.payload)
                .map_err(|_| SessionError::SessionUnusable)?;
            assert_eq!(change.selection.mount_key, None);
            self.sample_motion(0);
            return Ok(GatedCommandResult {
                command_id: 2,
                server_sequence: 2,
                status: CommandStatus::Accepted,
                payload: appearance::ChangeResult::Committed(2)
                    .encode()
                    .map_err(|_| SessionError::SessionUnusable)?,
            });
        }
        assert_eq!((command.capability, command.kind), (502, 6242));
        let &[8, mount @ (0 | 1)] = command.payload.as_slice() else {
            return Err(SessionError::SessionUnusable);
        };
        self.received.push(mount);
        if mount == 0 || (self.received == [1] && self.scenario == Scenario::DeltaBeforeReply) {
            self.sample_motion(0);
        }
        let sequence = self.received.len() as u64;
        Ok(GatedCommandResult {
            command_id: sequence,
            server_sequence: sequence,
            status: CommandStatus::Accepted,
            payload: Vec::new(),
        })
    }

    fn hotkey_own_actor(&self) -> Option<WorldSpatialEntity> {
        if self.received == [1] && !self.toggled_during_publish.replace(true) {
            // The reply has been handled and publish is about to replace the actor.
            assert_eq!(self.link.riding(), Some(false));
            self.link.mount_request();
        }
        Some(self.actor())
    }

    fn own_identity(&self) -> Option<[u8; 16]> {
        Some(self.actor().entity.identity)
    }

    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
        tokio::task::yield_now().await;
        if self.scenario != Scenario::DelayedMount {
            return self.serve_unmounted();
        }
        self.serve_delayed_mount()
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }

    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }
}

impl MountPeer<'_> {
    fn actor(&self) -> WorldSpatialEntity {
        WorldSpatialEntity {
            motion: Some(ActorMotion {
                mount_look_type: self.mount_look_type,
                ..ActorMotion::default()
            }),
            display_name: None,
            kind: EntityKind::Player,
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        }
    }

    fn serve_delayed_mount(&mut self) -> Result<(), SessionError> {
        match self.idle_turn {
            0 => {
                assert_eq!(self.link.riding(), Some(false));
                self.link.mount_request();
            }
            1 => {
                assert!(self.toggled_during_publish.get());
                assert_eq!(
                    self.received,
                    [1],
                    "toggle before publication sent another Mount"
                );
                assert_eq!(self.link.riding(), Some(false));
                self.link.mount_request();
                // The server may send the actor update after the accepted reply.
                self.sample_motion(139);
            }
            2 => {
                assert_eq!(self.received, [1]);
                assert_eq!(self.link.riding(), Some(true));
                self.link.mount_request();
            }
            3 => {
                assert_eq!(self.received, [1, 0]);
                assert_eq!(self.link.riding(), Some(false));
                self.link.request_logout();
            }
            _ => return Err(SessionError::SessionUnusable),
        }
        self.idle_turn += 1;
        Ok(())
    }

    fn serve_unmounted(&mut self) -> Result<(), SessionError> {
        assert_eq!(self.link.riding(), Some(false));
        let toggle_turn = if self.scenario == Scenario::NoMountOutfit {
            2
        } else {
            1
        };
        match self.idle_turn {
            0 => self.link.mount_request(),
            1 if self.scenario == Scenario::NoMountOutfit => self.request_no_mount_outfit()?,
            turn if turn == toggle_turn => {
                assert_eq!(self.received, [1]);
                self.link.mount_request();
            }
            turn if turn == toggle_turn + 1 => {
                assert_eq!(
                    self.received,
                    [1, 1],
                    "the next toggle must send exactly one Mount"
                );
                self.link.request_logout();
            }
            _ => return Err(SessionError::SessionUnusable),
        }
        self.idle_turn += 1;
        Ok(())
    }
}

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn mount_toggle_waits_for_the_session_to_publish_the_riding_actor() -> TestResult {
    run_mount_scenario(Scenario::DelayedMount)
}

#[test]
fn mount_toggle_recovers_when_no_mount_outfit_wins_before_visibility_refresh() -> TestResult {
    run_mount_scenario(Scenario::NoMountOutfit)
}

#[test]
fn mount_reply_releases_toggle_when_motion_arrived_before_the_reply() -> TestResult {
    run_mount_scenario(Scenario::DeltaBeforeReply)
}

fn run_mount_scenario(scenario: Scenario) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let (link, commands, outbox) = play_channel();
                let peer = MountPeer {
                    link: &link,
                    scenario,
                    events: Vec::new(),
                    mount_look_type: 0,
                    received: Vec::new(),
                    toggled_during_publish: Cell::new(false),
                    idle_turn: 0,
                };
                tokio::time::timeout(Duration::from_secs(2), run_session(peer, commands, outbox))
                    .await?;
                assert!(
                    std::iter::from_fn(|| link.poll())
                        .any(|event| matches!(event, PlayEvent::LoggedOut))
                );
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "mount session test panicked")?
}
