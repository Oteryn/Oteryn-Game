//! Current durable Character names for a spell owner's subsequent live lookup.
//!
//! Uses the existing Character roots and naming-policy namespace. A returned
//! snapshot proves a durable name binding only: it is not online presence,
//! current actor position, staff access or permission to disclose Exiva data.
//! Those facts must be independently resolved by their live owners before a
//! Find Person message is produced.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]

use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::character_name::CharacterName;
use crate::domain::{CharacterId, CharacterRevision, WorldId};
use crate::foundation::GameSessionId;
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedCharacterSnapshot {
    character_id: CharacterId,
    world_id: WorldId,
    revision: CharacterRevision,
    name: CharacterName,
    /// A candidate only. FreshAdmissionStore must independently reread it.
    session_candidate: Option<GameSessionId>,
}

impl NamedCharacterSnapshot {
    pub(crate) const fn session_candidate(&self) -> Option<GameSessionId> {
        self.session_candidate
    }
    pub(crate) const fn character_id(&self) -> CharacterId {
        self.character_id
    }
    pub(crate) const fn world_id(&self) -> WorldId {
        self.world_id
    }
    pub(crate) const fn revision(&self) -> CharacterRevision {
        self.revision
    }
    pub(crate) fn name(&self) -> &CharacterName {
        &self.name
    }
}

impl DurabilityRoot {
    /// A bounded source wildcard read under the actual caster owner's transaction.
    /// Returned sessions are candidates, never current presence or permissions.
    pub(crate) async fn lookup_spell_player_names_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        authority: &super::spell_item_transaction::SpellItemAuthority,
        input: &str,
    ) -> Result<Vec<NamedCharacterSnapshot>, super::spell_item_transaction::SpellItemError> {
        use super::spell_item_transaction::{SpellItemError, check_transaction};
        check_transaction(tx, authority).await?;
        let Some((query, prefix)) = source_name_query(input) else {
            return Ok(Vec::new());
        };
        let world = match authority.runtime_scope() {
            crate::foundation::RuntimeScopeRefV1::Channel { world_id, .. }
            | crate::foundation::RuntimeScopeRefV1::Instance { world_id, .. } => world_id,
        };
        let rows = sqlx::query("SELECT uuid_send(r.character_id) AS character_bytes,uuid_send(r.world_id) AS world_bytes,r.character_revision::text AS revision,r.name,r.name_key,uuid_send(s.game_session_id) AS session_candidate FROM game_character_roots r JOIN game_durability_admission_character_guards g USING(character_id) JOIN game_durability_reconnect_sessions s ON s.game_session_id=g.holder_game_session_id WHERE r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1 AND g.eligible AND g.world_id=r.world_id AND s.character_id=r.character_id AND s.world_id=r.world_id AND s.session_state=2 AND s.current_transport_ref IS NOT NULL AND s.control_loss_epoch IS NULL AND (($3 AND left(lower(r.name),length($2))=$2) OR (NOT $3 AND lower(r.name)=$2)) ORDER BY lower(r.name) LIMIT 2 FOR SHARE OF r,g,s")
            .bind(world.as_bytes().as_slice()).bind(&query).bind(prefix).fetch_all(&mut **tx).await?;
        rows.iter()
            .map(|row| {
                let name: String = row.try_get("name")?;
                let canonical =
                    CharacterName::parse(&name).map_err(|_| DurabilityError::InvalidStoredState)?;
                if (prefix && !name.to_ascii_lowercase().starts_with(&query))
                    || (!prefix && name.to_ascii_lowercase() != query)
                {
                    return Err(SpellItemError::Rejected("source name mismatch"));
                }
                let mut named = decode_name(
                    row.try_get("character_bytes")?,
                    row.try_get("world_bytes")?,
                    row.try_get("revision")?,
                    name,
                    row.try_get("name_key")?,
                    &canonical.comparison_key(),
                )?;
                let session: Vec<u8> = row.try_get("session_candidate")?;
                named.session_candidate = Some(
                    GameSessionId::decode(&session)
                        .map_err(|_| SpellItemError::Rejected("source name session"))?,
                );
                Ok(named)
            })
            .collect()
    }
    /// Read an exact naming-policy key in one World under the existing reconciled
    /// Character recovery authority. No second name registry is maintained.
    pub(crate) async fn lookup_character_name_for_spell(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: WorldId,
        requested_name: &CharacterName,
    ) -> Result<Option<NamedCharacterSnapshot>, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let world_bytes = *world.as_bytes();
        let requested_key = requested_name.comparison_key();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query(
                "SELECT uuid_send(r.character_id) AS character_bytes, uuid_send(r.world_id) AS world_bytes, \
                 r.character_revision::text AS revision, r.name, r.name_key, \
                 CASE WHEN g.eligible AND g.world_id = r.world_id THEN uuid_send(g.holder_game_session_id) ELSE NULL END AS session_candidate \
                 FROM game_character_roots r LEFT JOIN game_durability_admission_character_guards g USING (character_id) \
                 WHERE r.world_id = encode($1, 'hex')::uuid AND r.name_key = $2 AND r.lifecycle = 1 \
                 FOR SHARE OF r"
            ).bind(world_bytes.as_slice()).bind(&requested_key).fetch_optional(&mut *tx).await?;
            let named = row.as_ref().map(|row| -> Result<NamedCharacterSnapshot, DurabilityError> {
                let character: Vec<u8> = row.try_get("character_bytes")?;
                let world: Vec<u8> = row.try_get("world_bytes")?;
                let mut named = decode_name(character, world, row.try_get("revision")?,
                    row.try_get("name")?, row.try_get("name_key")?, &requested_key)?;
                let candidate: Option<Vec<u8>> = row.try_get("session_candidate")?;
                named.session_candidate = candidate.map(|bytes|
                    GameSessionId::decode(&bytes).map_err(|_| DurabilityError::InvalidStoredState)
                ).transpose()?;
                Ok(named)
            }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(named))
        })).await?
    }
}

fn source_name_query(input: &str) -> Option<(String, bool)> {
    if input.is_empty() || input.len() > 29 || !input.is_ascii() {
        return None;
    }
    if let Some(prefix) = input.strip_suffix('~') {
        if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ') {
            return None;
        }
        Some((prefix.to_ascii_lowercase(), true))
    } else {
        CharacterName::parse(input).ok()?;
        Some((input.to_ascii_lowercase(), false))
    }
}

fn decode_name(
    character: Vec<u8>,
    world: Vec<u8>,
    revision: String,
    name: String,
    stored_key: String,
    requested_key: &str,
) -> Result<NamedCharacterSnapshot, DurabilityError> {
    let invalid = || DurabilityError::InvalidStoredState;
    let character_id = CharacterId::from_bytes(character.try_into().map_err(|_| invalid())?)
        .map_err(|_| invalid())?;
    let world_id =
        WorldId::from_bytes(world.try_into().map_err(|_| invalid())?).map_err(|_| invalid())?;
    let revision =
        CharacterRevision::new(revision.parse().map_err(|_| invalid())?).map_err(|_| invalid())?;
    let name = CharacterName::parse(&name).map_err(|_| invalid())?;
    if name.comparison_key() != stored_key || stored_key != requested_key {
        return Err(invalid());
    }
    Ok(NamedCharacterSnapshot {
        character_id,
        world_id,
        revision,
        name,
        session_candidate: None,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn wildcard_query_preserves_source_spaces_and_bounded_exact_spelling() {
        assert_eq!(source_name_query("Al D~"), Some(("al d".into(), true)));
        assert_eq!(
            source_name_query("Al Dric"),
            Some(("al dric".into(), false))
        );
        for bad in ["", "~", "Al%~", "Al_~", "A~B", "Ąl~"] {
            assert!(source_name_query(bad).is_none());
        }
        assert!(source_name_query(&"a".repeat(30)).is_none());
    }

    fn bytes(raw: u8) -> Vec<u8> {
        let mut bytes = vec![0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = raw;
        bytes
    }

    #[test]
    fn durable_name_decoder_keeps_native_case_but_checks_exact_policy_key() {
        let named = decode_name(
            bytes(1),
            bytes(2),
            "7".into(),
            "Al Dric".into(),
            "aldric".into(),
            "aldric",
        )
        .unwrap();
        assert_eq!(named.name().as_str(), "Al Dric");
        assert_eq!(named.revision(), CharacterRevision::new(7).unwrap());
        assert_eq!(
            named.world_id(),
            WorldId::from_bytes(bytes(2).try_into().unwrap()).unwrap()
        );
        assert_eq!(
            named.character_id(),
            CharacterId::from_bytes(bytes(1).try_into().unwrap()).unwrap()
        );
    }

    #[test]
    fn malformed_or_substituted_durable_name_bindings_refuse() {
        for (revision, name, stored, requested) in [
            ("0", "Al Dric", "aldric", "aldric"),
            ("7", "Al  Dric", "aldric", "aldric"),
            ("7", "Al Dric", "other", "aldric"),
            ("7", "Al Dric", "aldric", "other"),
        ] {
            assert!(
                decode_name(
                    bytes(1),
                    bytes(2),
                    revision.into(),
                    name.into(),
                    stored.into(),
                    requested
                )
                .is_err()
            );
        }
        assert!(
            decode_name(
                vec![1],
                bytes(2),
                "7".into(),
                "Al Dric".into(),
                "aldric".into(),
                "aldric"
            )
            .is_err()
        );
    }
}
