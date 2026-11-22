use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{PlayEvent, PlayLink, Stepper, play_channel, run_session},
};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_session::{
    ActorPosition, CommandStatus, EntityDetail, EntityKind, EntityRef, GatedCommandResult,
    LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome, WorldSpatialEntity,
    actor_motion::ActorMotion,
};
use std::{cell::Cell, time::Duration};

struct MountPeer<'a> {
    link: &'a PlayLink,
    mount_look_type: u32,
    received: Vec<u8>,
    toggled_during_publish: Cell<bool>,
    idle_turn: u8,
}

impl Stepper for MountPeer<'_> {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 502
    }

    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        assert_eq!((command.capability, command.kind), (502, 6242));
        let &[8, mount @ (0 | 1)] = command.payload.as_slice() else {
            return Err(SessionError::SessionUnusable);
        };
        self.received.push(mount);
        if mount == 0 {
            self.mount_look_type = 0;
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
        Some(WorldSpatialEntity {
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
        })
    }

    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
        tokio::task::yield_now().await;
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
                self.mount_look_type = 139;
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
        Vec::new()
    }
}

#[test]
fn mount_toggle_waits_for_the_session_to_publish_the_riding_actor()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let (link, commands, outbox) = play_channel();
                let peer = MountPeer {
                    link: &link,
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
