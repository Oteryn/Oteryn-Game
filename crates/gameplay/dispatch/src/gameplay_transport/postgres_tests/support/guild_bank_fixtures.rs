#[path = "guild_bank/seed.rs"]
pub(super) mod bank_seed;

#[path = "guild_bank/transport.rs"]
pub(super) mod transport;

use super::fixture::{self, Harness, debug, id};

use super::TestResult;

use crate::gameplay_transport::connection::AdmittedSession;

use crate::gameplay_transport::ComposedFreshAdmission;

use oteryn_protocol_oteryn::guild_bank::*;

use oteryn_protocol_oteryn::{CommandStatus, guild as guild_wire};

use transport::exchange;

pub(super) async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    guild: [u8; 16],
) -> TestResult<GuildBankReply> {
    let id = s.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        s,
        id,
        COMMAND_TYPE_GUILD_BANK_QUERY,
        &encode_guild_bank_query(&GuildBankQuery {
            guild: Some(guild),
            before: None,
            after_claim: None,
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    decode_guild_bank_reply(&bytes).map_err(debug)
}

pub(super) async fn guild(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    s: &mut AdmittedSession,
    action: guild_wire::GuildIntent,
) -> TestResult<guild_wire::GuildIntentResult> {
    let id = s.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        s,
        id,
        guild_wire::COMMAND_TYPE_GUILD_INTENT,
        &guild_wire::encode_guild_intent(&action).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let r = guild_wire::decode_guild_intent_result(&bytes).map_err(debug)?;
    assert_eq!(r.code, 1);
    Ok(r)
}
