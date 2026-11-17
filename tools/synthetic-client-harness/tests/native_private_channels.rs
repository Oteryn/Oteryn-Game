use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_ui::chat_private_channel::{Intent, panel_text};
use oteryn_session::{
    ChatChannelList, ChatDisposition, ChatIntent, ChatLine, ChatOutcome, ChatRoomSet,
    InvitedChannel, PrivateChannelIntent, PrivateChannelLine,
};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    commands: Arc<Mutex<Vec<ChatIntent>>>,
    list: ChatChannelList,
    lines: Vec<ChatLine>,
}

impl Stepper for Peer {
    fn chat_private_state(&self) -> (bool, Option<ChatChannelList>) {
        (true, Some(self.list.clone()))
    }
    fn chat_state(&self) -> Option<(ChatRoomSet, Vec<ChatLine>)> {
        Some((ChatRoomSet::default(), self.lines.clone()))
    }
    async fn chat_public(&mut self, intent: &ChatIntent) -> Result<ChatOutcome, SessionError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(intent.clone());
        match intent {
            ChatIntent::PrivateChannel(PrivateChannelIntent::Create) => {
                self.list.own_channel = Some(100)
            }
            ChatIntent::PrivateChannel(PrivateChannelIntent::Say { channel_id, text }) => self
                .lines
                .push(ChatLine::PrivateChannel(PrivateChannelLine {
                    channel_id: *channel_id,
                    speaker_name: "Player One".into(),
                    text: text.clone(),
                })),
            ChatIntent::PrivateChannel(PrivateChannelIntent::Leave { .. }) => {
                self.list.invited.clear()
            }
            ChatIntent::PrivateChannel(_) => {}
            _ => return Err(SessionError::SessionUnusable),
        }
        Ok(ChatOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: ChatDisposition::Ok,
            wait_seconds: 0,
            result_server_sequence: 1,
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

async fn wait_for(
    link: &PlayLink,
    predicate: impl Fn(&oteryn_client_ui::chat_private_channel::ChatPanel) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if predicate(&link.chat_private_panel()) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[test]
fn private_channel_commands_and_live_state_reach_the_chat_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Peer {
                    commands: Arc::clone(&commands),
                    list: ChatChannelList {
                        invited: vec![InvitedChannel {
                            channel_id: 101,
                            owner_name: "Friend".into(),
                        }],
                        ..Default::default()
                    },
                    lines: vec![ChatLine::PrivateChannel(PrivateChannelLine {
                        channel_id: 101,
                        speaker_name: "Friend".into(),
                        text: "welcome".into(),
                    })],
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                wait_for(&link, |panel| panel.selected).await?;
                assert!(panel_text(&link.chat_private_panel()).contains("Friend: welcome"));
                link.chat_private_select(101);
                link.chat_private_request(Intent::Join);
                wait_for(&link, |panel| !panel.pending).await?;
                link.chat_private_text("hello channel".into());
                link.chat_private_request(Intent::Send);
                wait_for(&link, |panel| {
                    !panel.pending && panel_text(panel).contains("Player One: hello channel")
                })
                .await?;
                link.chat_private_request(Intent::Create);
                wait_for(&link, |panel| {
                    !panel.pending
                        && panel
                            .list
                            .as_ref()
                            .is_some_and(|list| list.own_channel == Some(100))
                })
                .await?;
                link.chat_private_name("Guest".into());
                link.chat_private_request(Intent::Invite);
                wait_for(&link, |panel| !panel.pending).await?;
                link.chat_private_request(Intent::Exclude);
                wait_for(&link, |panel| !panel.pending).await?;
                link.chat_private_request(Intent::Leave);
                wait_for(&link, |panel| !panel.pending && panel.channel == Some(100)).await?;
                link.chat_private_request(Intent::List);
                wait_for(&link, |panel| !panel.pending).await?;
                assert_eq!(
                    *commands.lock().unwrap_or_else(PoisonError::into_inner),
                    vec![
                        ChatIntent::PrivateChannel(PrivateChannelIntent::Join { channel_id: 101 }),
                        ChatIntent::PrivateChannel(PrivateChannelIntent::Say {
                            channel_id: 101,
                            text: "hello channel".into()
                        }),
                        ChatIntent::PrivateChannel(PrivateChannelIntent::Create),
                        ChatIntent::PrivateChannel(PrivateChannelIntent::Invite {
                            name: "Guest".into()
                        }),
                        ChatIntent::PrivateChannel(PrivateChannelIntent::Exclude {
                            name: "Guest".into()
                        }),
                        ChatIntent::PrivateChannel(PrivateChannelIntent::Leave { channel_id: 101 }),
                        ChatIntent::PrivateChannel(PrivateChannelIntent::List),
                    ]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })
        })?
        .join()
        .map_err(|_| "private chat UI test thread panicked")?
}
