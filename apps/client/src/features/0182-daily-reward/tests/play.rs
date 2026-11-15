use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, DailyRewardResponse, daily_reward,
};
use oteryn_client_ui::daily_reward::{Intent, panel_text};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    commands: Arc<Mutex<Vec<DailyRewardCommand>>>,
    claimed: bool,
    returner_offer: Option<[u8; 16]>,
}
impl Stepper for Peer {
    fn daily_reward_selected(&self) -> bool {
        true
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        if command.kind == daily_reward::details::COMMAND_TYPE {
            assert_eq!(command.capability, 182);
            return Ok(oteryn_session::GatedCommandResult {command_id:2,server_sequence:2,status:CommandStatus::Accepted,payload:daily_reward::details::Details::empty(1).encode().map_err(|_|SessionError::SessionUnusable)?});
        }
        assert_eq!(
            (command.capability, command.kind),
            (182, daily_reward::returner::QUERY)
        );
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: daily_reward::returner::Reply {
                state: if self.returner_offer.is_some() { 1 } else { 0 },
                id: self.returner_offer,
            }
            .encode()
            .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn daily_reward(
        &mut self,
        command: &DailyRewardCommand,
    ) -> Result<DailyRewardResponse, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(*command);
        let reply = match command {
            DailyRewardCommand::Query => DailyRewardReply::Wall(daily_reward::DailyRewardWall {
                lane_day: 12,
                streak: if self.claimed { 14 } else { 13 },
                jokers: 2,
                claimed: self.claimed,
                claim_enabled: !self.claimed,
            }),
            DailyRewardCommand::Claim => {
                self.claimed = true;
                DailyRewardReply::Result(daily_reward::DailyRewardResult {
                    disposition: 1,
                    transaction_id: Some([0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]),
                })
            }
        };
        Ok(DailyRewardResponse {
            status: CommandStatus::Accepted,
            reply: Some(reply),
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
            let p = link.daily_reward_panel();
            if p.selected && !p.pending && (!loaded || p.wall.is_some()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn daily_reward_query_and_claim_drive_the_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (peer_commands, peer) = (
                    Arc::clone(&commands),
                    Peer {
                        commands,
                        claimed: false,
                        returner_offer: None,
                    },
                );
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, false).await?;
                assert!(
                    include_str!("../../../windows_shell/input.rs")
                        .contains("play.link.daily_reward_input(events)")
                );
                assert!(link.daily_reward_input(&[NormalizedInputEvent::Key {
                    code: KeyCode::new(7)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control),
                    repeat: false,
                }]));
                assert!(link.daily_reward_panel().open);
                ready(&link, true).await?;
                assert!(panel_text(&link.daily_reward_panel()).contains("Day 5 of 7"));
                let text = panel_text(&link.daily_reward_panel());
                assert!(text.contains(
                    "Your reward streak currently grants you a 7% chance to gain double weapon proficiency XP."
                ));
                assert!(text.contains("Claim your daily reward once between two regular server saves"));
                link.daily_reward_request(Intent::Claim);
                tokio::time::timeout(Duration::from_secs(3), async {
                    loop {
                        let p = link.daily_reward_panel();
                        if !p.pending && p.wall.is_some_and(|wall| wall.streak == 14) && p.details.is_some() { break; }
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert_eq!(
                    *peer_commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![DailyRewardCommand::Query, DailyRewardCommand::Claim, DailyRewardCommand::Query]
                );
                assert_eq!(
                    link.daily_reward_panel().feedback,
                    Some("Daily reward claimed")
                );
                assert!(panel_text(&link.daily_reward_panel()).contains("Status: claimed"));
                link.daily_reward_request(Intent::Claim);
                assert_eq!(
                    peer_commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    3
                );
                link.daily_reward_request(Intent::Refresh);
                ready(&link, true).await?;
                assert!(
                    link.daily_reward_panel()
                        .wall
                        .is_some_and(|wall| wall.claimed && !wall.claim_enabled)
                );
                assert!(panel_text(&link.daily_reward_panel())
                    .contains("grants you a 7% chance to gain double weapon proficiency XP"));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Daily reward UI test thread panicked")?
}
#[path = "details.rs"]
mod details;

#[path = "returner.rs"]
mod returner;

#[path = "../native.rs"]
mod native_dispatch;

#[test]
fn native_reward_actions_are_guarded_while_dead() -> Result<(), Error> {
    #[cfg(windows)]
    let _open: fn(&PlayLink) = native_dispatch::open_window;
    let (link, mut commands, outbox) = play_channel();
    outbox.daily_reward_initialize(true);
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    mailbox.death.dead = true;
    mailbox.daily_reward.wall = Some(daily_reward::DailyRewardWall {
        lane_day: 1,
        streak: 1,
        jokers: 0,
        claimed: false,
        claim_enabled: true,
    });
    for intent in [Intent::Refresh, Intent::Claim] {
        native_dispatch::enqueue_action(&mut mailbox, &link.commands, intent);
        assert!(!mailbox.daily_reward.pending);
        assert!(commands.try_recv().is_err());
    }
    mailbox.death.dead = false;
    native_dispatch::enqueue_action(&mut mailbox, &link.commands, Intent::Refresh);
    assert!(mailbox.daily_reward.pending);
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::DailyReward(DailyRewardCommand::Query)
    ));
    Ok(())
}

#[test]
fn successful_reward_claim_requeries_wall_and_authoritative_history() -> Result<(), Error> {
    let (link, mut commands, outbox) = play_channel();
    outbox.daily_reward_initialize(true);
    outbox.daily_reward_result(&DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(DailyRewardReply::Wall(daily_reward::DailyRewardWall {
            lane_day: 1,
            streak: 1,
            jokers: 1,
            claimed: false,
            claim_enabled: true,
        })),
    });
    {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        m.daily_reward.details_attempted = true;
        m.daily_reward.details = Some(daily_reward::details::Details::empty(1));
    }
    link.daily_reward_request(Intent::Claim);
    commands.try_recv()?;
    outbox.daily_reward_result(&DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(DailyRewardReply::Result(daily_reward::DailyRewardResult {
            disposition: 1,
            transaction_id: Some([0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 7]),
        })),
    });
    assert!(matches!(
        outbox.death_command(&mut commands)?,
        crate::combat_input::PlayCommand::DailyReward(DailyRewardCommand::Query)
    ));
    outbox.daily_reward_result(&DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(DailyRewardReply::Wall(daily_reward::DailyRewardWall {
            lane_day: 1,
            streak: 2,
            jokers: 0,
            claimed: true,
            claim_enabled: false,
        })),
    });
    assert!(
        matches!(outbox.death_command(&mut commands)?,crate::combat_input::PlayCommand::Feature(c) if (c.capability,c.kind) == (182,738))
    );
    let mut details = daily_reward::details::Details::empty(2);
    details.history[0] = Some(daily_reward::details::Receipt {
        epoch: 2,
        reset_day: 1,
        lane_day: 1,
        streak: 2,
        jokers_spent: 1,
        own_character: true,
    });
    outbox.daily_reward_details_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: details.encode().map_err(|_| "details")?,
    });
    let p = link.daily_reward_panel();
    assert_eq!(p.wall.map(|w| (w.streak, w.jokers)), Some((2, 0)));
    assert_eq!(p.details, Some(details));
    assert_eq!(p.feedback, Some("Daily reward claimed"));
    assert!(!p.pending);
    assert!(outbox.death_command(&mut commands).is_err());
    {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        native_dispatch::enqueue_action(&mut m, &link.commands, Intent::Refresh);
        assert!(!m.daily_reward.details_attempted);
    }
    Ok(())
}
