//! Typed guild requests over capability 40.
pub use oteryn_session::{CommandStatus, guild};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildCommand {
    Intent(guild::GuildIntent),
    Query(guild::GuildQuery),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildReply {
    Intent {
        status: CommandStatus,
        result: Option<guild::GuildIntentResult>,
    },
    Query {
        status: CommandStatus,
        result: Option<guild::GuildQueryResult>,
    },
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &GuildCommand,
) -> Result<GuildReply, SessionError> {
    let (kind, payload) = match command {
        GuildCommand::Intent(value) => (
            guild::COMMAND_TYPE_GUILD_INTENT,
            guild::encode_guild_intent(value),
        ),
        GuildCommand::Query(value) => (
            guild::COMMAND_TYPE_GUILD_QUERY,
            guild::encode_guild_query(value),
        ),
    };
    let payload = payload.map_err(|_| SessionError::CapabilityNotSelected {
        capability: guild::CAPABILITY_GUILD_V1,
    })?;
    let reply = session
        .gated_command(guild::CAPABILITY_GUILD_V1, kind, &payload)
        .await?;
    let accepted = matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    match command {
        GuildCommand::Intent(_) => Ok(GuildReply::Intent {
            status: reply.status,
            result: accepted
                .then(|| guild::decode_guild_intent_result(&reply.payload))
                .transpose()
                .map_err(|_| SessionError::SessionUnusable)?,
        }),
        GuildCommand::Query(_) => Ok(GuildReply::Query {
            status: reply.status,
            result: accepted
                .then(|| guild::decode_guild_query_result(&reply.payload))
                .transpose()
                .map_err(|_| SessionError::SessionUnusable)?,
        }),
    }
}
