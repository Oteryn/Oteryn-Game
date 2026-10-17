//! GUILD-WIRE-1 untrusted request normalization only. No production module registration,
//! capability offer, role/current-fence predicate, guild writer or dispatch is installed here.
//! The CP integration must bind real sequenced CommandRef/recovery/session/World/Account,
//! replay before live mutation, and the actual GUILD0-LO-01 transaction owners independently.
#![allow(
    dead_code,
    reason = "Owned wire preparation; native transport registration is CP-only"
)]

use crate::chat::ChatText;
use crate::domain::character_name::CharacterName;
use oteryn_protocol_oteryn::guild::{
    GuildIdentity, GuildIntent, GuildQuery, GuildSection, GuildWireError, decode_guild_intent,
    decode_guild_query, encode_guild_intent,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuildInputError {
    Wire(GuildWireError),
    NameInvalid,
    TextInvalid,
    CursorBinding,
    AccountWorldClaimsContextRequired,
}
impl From<GuildWireError> for GuildInputError {
    fn from(value: GuildWireError) -> Self {
        Self::Wire(value)
    }
}

/// Request DATA, never an authenticated/current actor, membership or authorization proof.
pub(crate) struct GuildRequestData {
    pub(crate) intent: GuildIntent,
    /// Canonical original decoded body BEFORE normalization. Thus two distinct texts that trim
    /// to the same display text still conflict if reused under the same occurrence.
    /// The native writer must bind this digest with its own real occurrence/actor/World context;
    /// it is not a newly minted operation ID, receipt, or a replacement for the owner's binding.
    pub(crate) body_digest: [u8; 32],
}
impl std::fmt::Debug for GuildRequestData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuildRequestData").finish_non_exhaustive()
    }
}
fn name(value: &str) -> Result<String, GuildInputError> {
    CharacterName::parse(value)
        .map(|value| value.as_str().to_owned())
        .map_err(|_| GuildInputError::NameInvalid)
}
fn message(value: &str, max_characters: usize) -> Result<String, GuildInputError> {
    // Empty metadata is represented explicitly by the body. It supplies no permission to edit.
    if value.is_empty() {
        return Ok(String::new());
    }
    let text = ChatText::parse(value).map_err(|_| GuildInputError::TextInvalid)?;
    if text.as_str().chars().count() > max_characters {
        return Err(GuildInputError::TextInvalid);
    }
    Ok(text.as_str().to_owned())
}
pub(crate) fn decode_guild_request(payload: &[u8]) -> Result<GuildRequestData, GuildInputError> {
    let mut intent = decode_guild_intent(payload)?;
    let canonical = encode_guild_intent(&intent)?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn:guild-intent-body:v1\0");
    hash.update(&canonical);
    let body_digest = hash.finalize().into();
    match &mut intent {
        GuildIntent::Found { name: value } | GuildIntent::Disband { confirm: value } => {
            *value = name(value)?
        }
        GuildIntent::Invite { character }
        | GuildIntent::RevokeInvite { character }
        | GuildIntent::Exclude { character }
        | GuildIntent::SetRank { character, .. } => *character = name(character)?,
        GuildIntent::SetTitle {
            character, title, ..
        } => {
            *character = name(character)?;
            *title = message(title, 29)?;
        }
        GuildIntent::SetMessage { text, .. } => *text = message(text, 255)?,
        GuildIntent::Resign { successor } => *successor = name(successor)?,
        GuildIntent::EditRanks { .. } => {
            // Accepted rank count/transport size is already checked by the codec. GUILD0 does
            // not assign a new rank-name grammar; the writer must use its canonical 0022 name_key
            // implementation and atomically enforce uniqueness. Do not copy a second normalizer.
        }
        GuildIntent::Accept { .. }
        | GuildIntent::Leave
        | GuildIntent::Deposit { .. }
        | GuildIntent::Withdraw { .. }
        | GuildIntent::ClaimDisbandPayout { .. } => {}
    }
    Ok(GuildRequestData {
        intent,
        body_digest,
    })
}

pub(crate) fn decode_guild_query_data(payload: &[u8]) -> Result<GuildQuery, GuildInputError> {
    let mut query = decode_guild_query(payload)?;
    if let Some(public) = &mut query.public_guild {
        *public = name(public)?;
    }
    Ok(query)
}

/// Structural cursor binding after the real query owner independently resolves its selected
/// guild and authorizes the section. Supplying a GuildIdentity here cannot authorize a read.
pub(crate) fn check_resolved_cursor(
    query: &GuildQuery,
    resolved: GuildIdentity,
) -> Result<(), GuildInputError> {
    if let Some(cursor) = &query.cursor {
        // Account-wide claim pagination has no single selected Guild. Its genuine current
        // Account+World query-owner context and cursor acceptance contract remain a CP seam.
        // Do not turn a client-supplied GuildId into that missing context.
        if query.section == GuildSection::DisbandClaims {
            return Err(GuildInputError::AccountWorldClaimsContextRequired);
        }
        cursor
            .check_binding(resolved, query.section)
            .map_err(|_| GuildInputError::CursorBinding)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "guild_intent_tests.rs"]
mod tests;
