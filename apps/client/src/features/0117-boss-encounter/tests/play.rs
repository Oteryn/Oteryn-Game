use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use oteryn_client_ui::boss_encounter::panel_text;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
type Commands = Arc<Mutex<Vec<(u32, Vec<u8>)>>>;
struct Peer {
    commands: Commands,
}
impl Stepper for Peer {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 117
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((command.kind, command.payload.clone()));
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: vec![8, 100, 34, 7, 10, 3, b'a', b':', b'b', 16, 3, 40, 5,
                58, 8, 8, 5, 16, 3, 24, 15, 32, 1],
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}
async fn ready(link: &PlayLink, loaded: bool) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let p = link.boss_encounter_panel();
            if p.selected && !p.pending && (!loaded || p.snapshot.is_some()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn boss_encounter_play_input_queries_and_displays_server_snapshot() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        commands: Arc::clone(&commands),
                    },
                    incoming,
                    outbox,
                ));
                ready(&link, false).await?;
                let event = NormalizedInputEvent::Key {
                    code: KeyCode::new(13)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::NONE
                        .with(Modifier::Control)
                        .with(Modifier::Shift),
                    repeat: false,
                };
                assert!(link.boss_encounter_input(&[event]));
                ready(&link, true).await?;
                let text = panel_text(&link.boss_encounter_panel());
                assert!(text.contains("a:b: 3 kills"));
                assert!(text.contains("Boss points: 5"));
                assert!(text.contains("Soul War taints: 5"));
                assert!(text.contains("Rotten Blood taints: 3; final taint: yes"));
                assert!(text.contains("Rotten Blood bosses defeated: Ichgahal Murcion Vemiath Chagorz"));
                assert!(link.boss_encounter_panel().open);
                assert_eq!(
                    *commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![(133, vec![16, 1])]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "boss status test thread panicked")?
}

#[path = "activity.rs"]
mod activity;

#[test]
fn cooldown_shortcut_opens_only_the_tracker_and_queries_real_command() -> Result<(), Error> {
    let (link, mut commands, outbox) = play_channel();
    outbox.boss_encounter_initialize(true);
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(13)?, state: ButtonState::Pressed,
        modifiers: Modifiers::NONE.with(Modifier::Control).with(Modifier::Alt).with(Modifier::Shift), repeat: false,
    };
    assert!(link.boss_encounter_input(&[event]));
    assert!(link.boss_encounter_panel().cooldowns_open);
    assert!(!link.boss_encounter_panel().open);
    assert!(matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::Feature(FeatureCommand { capability: 117, kind: 133, .. }))));
    Ok(())
}
