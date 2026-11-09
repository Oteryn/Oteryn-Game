//! Preserve legacy keys and bind repeatable World rewards to a locked quest epoch.
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::foundation::CommandRef;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
pub(crate) enum QuestMintClaim<'a> {
    Once,
    PaidExchange,
    WorldUse,
    Cycle { track: &'a str, value: i64 },
}

pub(super) fn key(
    character: CharacterId,
    command: CommandRef,
    callback: &str,
    claim: QuestMintClaim<'_>,
) -> Result<String, DurabilityError> {
    let (prefix, digest) = match claim {
        QuestMintClaim::Once => return Ok(callback.to_owned()),
        QuestMintClaim::PaidExchange => (
            "oteryn:quest-exchange/",
            Sha256::new()
                .chain_update(b"oteryn:quest-exchange:v1\0")
                .chain_update(character.as_bytes())
                .chain_update(command.game_session_id().as_bytes())
                .chain_update(command.command_id().get().to_string().as_bytes())
                .chain_update(b":")
                .chain_update(callback.as_bytes())
                .finalize(),
        ),
        QuestMintClaim::WorldUse => (
            "oteryn:quest-world-use/",
            Sha256::new()
                .chain_update(b"oteryn:quest-world-use:v1\0")
                .chain_update(character.as_bytes())
                .chain_update(command.game_session_id().as_bytes())
                .chain_update(command.command_id().get().to_string().as_bytes())
                .chain_update(b":")
                .chain_update(callback.as_bytes())
                .finalize(),
        ),
        QuestMintClaim::Cycle { track, value } => {
            if value < 0 {
                return Err(DurabilityError::InvalidStoredState);
            }
            // The tuple is unambiguous and does not contain a command or session nonce.
            let cycle = serde_json::to_vec(&(callback, track, value))
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            (
                "oteryn:quest-cycle/",
                Sha256::new()
                    .chain_update(b"oteryn:quest-cycle:v1\0")
                    .chain_update(character.as_bytes())
                    .chain_update(cycle)
                    .finalize(),
            )
        }
    };
    Ok(format!(
        "{prefix}{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{CommandId, GameSessionId};

    #[test]
    fn world_cycle_keys_ignore_commands_but_bind_character_source_track_and_epoch()
    -> Result<(), String> {
        let id = |seed| {
            [
                seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
            ]
        };
        let character = CharacterId::from_bytes(id(41)).map_err(|e| format!("{e:?}"))?;
        let command = |session, n| -> Result<_, String> {
            Ok(CommandRef::new(
                GameSessionId::decode(&id(session)).map_err(|e| format!("{e:?}"))?,
                CommandId::new(n).map_err(|e| format!("{e:?}"))?,
            ))
        };
        let scoped = |who, session, n, source, track, value| {
            key(
                who,
                command(session, n)?,
                source,
                QuestMintClaim::Cycle { track, value },
            )
            .map_err(|e| format!("{e:?}"))
        };
        let first = scoped(character, 50, 1, "source", "epoch", 0)?;
        assert_eq!(first, scoped(character, 51, 999, "source", "epoch", 0)?);
        assert_ne!(first, scoped(character, 50, 1, "source", "epoch", 1)?);
        assert_ne!(first, scoped(character, 50, 1, "other", "epoch", 0)?);
        assert_ne!(first, scoped(character, 50, 1, "source", "other", 0)?);
        assert_ne!(
            first,
            scoped(
                CharacterId::from_bytes(id(42)).map_err(|e| format!("{e:?}"))?,
                50,
                1,
                "source",
                "epoch",
                0
            )?
        );
        assert!(scoped(character, 50, 1, "source", "epoch", -1).is_err());
        let per_use=key(character,command(50,1)?,"source",QuestMintClaim::WorldUse)
            .map_err(|e|format!("{e:?}"))?;
        assert_eq!(per_use,key(character,command(50,1)?,"source",QuestMintClaim::WorldUse).map_err(|e|format!("{e:?}"))?);
        for (who,session,n,source) in [
            (character,50,2,"source"),(character,51,1,"source"),(character,50,1,"other"),
            (CharacterId::from_bytes(id(42)).map_err(|e|format!("{e:?}"))?,50,1,"source"),
        ] {
            assert_ne!(per_use,key(who,command(session,n)?,source,QuestMintClaim::WorldUse).map_err(|e|format!("{e:?}"))?);
        }
        assert_ne!(per_use,key(character,command(50,1)?,"source",QuestMintClaim::PaidExchange).map_err(|e|format!("{e:?}"))?);
        assert_eq!(
            key(character, command(50, 1)?, "source", QuestMintClaim::Once)
                .map_err(|e| format!("{e:?}"))?,
            "source"
        );
        assert_eq!(
            key(
                character,
                command(50, 1)?,
                "source",
                QuestMintClaim::PaidExchange
            )
            .map_err(|e| format!("{e:?}"))?,
            "oteryn:quest-exchange/60c01506ec37ab38d432b8fb9e77eb91954b5978623492d61bc265a5c091a789",
            "existing paid exchanges keep their exact claim identity"
        );
        Ok(())
    }
}
