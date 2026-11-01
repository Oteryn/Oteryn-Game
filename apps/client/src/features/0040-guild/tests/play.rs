use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::guild::{GuildCommand, GuildReply, guild};
use oteryn_client_ui::guild::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
fn id() -> [u8; 16] {
    let mut id = [7; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
struct Peer;
impl Stepper for Peer {
    fn guild_selected(&self) -> bool {
        true
    }
    async fn guild(&mut self, command: &GuildCommand) -> Result<GuildReply, SessionError> {
        match command {
            GuildCommand::Query(query) => {
                let bytes =
                    guild::encode_guild_query(query).map_err(|_| SessionError::SessionUnusable)?;
                let decoded =
                    guild::decode_guild_query(&bytes).map_err(|_| SessionError::SessionUnusable)?;
                let result = match decoded {
                    guild::GuildQuery::Own => {
                        guild::GuildQueryResult::Own(Some(guild::GuildSummary {
                            guild: id(),
                            name: "Guild Name".into(),
                            state: 1,
                            revision: 3,
                            own_rank: 1,
                            title: "".into(),
                            message: "".into(),
                            ranks: vec!["Leader".into(), "Vice Leader".into(), "Member".into()],
                        }))
                    }
                    guild::GuildQuery::Roster { guild, .. } => guild::GuildQueryResult::Roster {
                        guild,
                        revision: 3,
                        members: vec![guild::GuildMember {
                            character: id(),
                            name: "Alice".into(),
                            level: 3,
                            title: "Member".into(),
                        }],
                        next: None,
                    },
                    guild::GuildQuery::Invitations => guild::GuildQueryResult::Invitations(vec![]),
                };
                let reply = guild::encode_guild_query_result(&result)
                    .map_err(|_| SessionError::SessionUnusable)?;
                Ok(GuildReply::Query {
                    status: CommandStatus::Accepted,
                    result: Some(
                        guild::decode_guild_query_result(&reply)
                            .map_err(|_| SessionError::SessionUnusable)?,
                    ),
                })
            }
            GuildCommand::Intent(intent) => {
                let bytes = guild::encode_guild_intent(intent)
                    .map_err(|_| SessionError::SessionUnusable)?;
                guild::decode_guild_intent(&bytes).map_err(|_| SessionError::SessionUnusable)?;
                let value = guild::GuildIntentResult {
                    code: 1,
                    guild: id(),
                    revision: Some(4),
                };
                let reply = guild::encode_guild_intent_result(&value)
                    .map_err(|_| SessionError::SessionUnusable)?;
                Ok(GuildReply::Intent {
                    status: CommandStatus::Accepted,
                    result: Some(
                        guild::decode_guild_intent_result(&reply)
                            .map_err(|_| SessionError::SessionUnusable)?,
                    ),
                })
            }
        }
    }
    async fn serve(&mut self, d: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(d).await;
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
async fn ready(link: &PlayLink, text: &str) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let p = link.guild_panel();
            if p.selected && !p.pending && panel_text(&p).contains(text) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn guild_controls_use_real_codecs_and_apply_server_state() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer, incoming, outbox));
                ready(&link, "No guild").await?;
                link.guild_request(Intent::RefreshOwn);
                ready(&link, "Guild Name").await?;
                link.guild_request(Intent::RefreshRoster);
                ready(&link, "Alice").await?;
                link.guild_request(Intent::Invite("Bob".into()));
                ready(&link, "accepted").await?;
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Guild UI test thread panicked")?
}
