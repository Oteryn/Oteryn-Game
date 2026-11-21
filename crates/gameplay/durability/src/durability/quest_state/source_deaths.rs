//! Captured native source eligibility is evidence, never current mutation authority.
//! Crystal 201189f HeartBossDeath:26-138; Court Warlock:323-329;
//! MagmaBubbleDeath:12-23; GoblinLeaderDeath:3-8. Current fences remain independently checked.
//! Herald additionally binds the qualified most-damage character captured at death.
use super::*;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDeathContext {
    pub scope: RuntimeScopeRefV1,
    pub scope_generation: u64,
    pub character: [u8; 16],
    pub session: GameSessionId,
    pub lease_generation: u64,
    pub position: [i32; 3],
    pub dealt_damage: bool,
    /// Present most-damage identity frozen by the native lethal owner, if required.
    pub top_damage_character: Option<[u8; 16]>,
}

impl SourceDeathContext {
    pub(super) fn bind(&self, binding: &mut [u8; 33]) {
        let mut digest = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"source-quest-death-v1")
            .chain_update(self.scope.world_id().as_bytes());
        match self.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                digest.update([1]);
                digest.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                digest.update([2]);
                digest.update(instance_id);
            }
        }
        digest.update(self.scope_generation.to_be_bytes());
        digest.update(self.character);
        digest.update(self.session.as_bytes());
        digest.update(self.lease_generation.to_be_bytes());
        for coordinate in self.position {
            digest.update(coordinate.to_be_bytes());
        }
        digest.update([u8::from(self.dealt_damage)]);
        // Preserve every pre-existing source receipt byte when this proof is absent.
        if let Some(character) = self.top_damage_character {
            digest.update(b"source-top-damage-v1");
            digest.update(character);
        }
        binding[1..].copy_from_slice(&digest.finalize());
    }
    pub(super) async fn admit(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: CurrentCharacterGameplayFence,
        death: &super::native_kills::NativeQuestDeath,
    ) -> StoredResult<bool> {
        let Some(rule) = oteryn_gameplay_quests::quest::source_deaths::rule(&death.transition)
        else {
            return Ok(false);
        };
        if self.scope != fence.runtime_scope
            || self.scope_generation != fence.scope_ownership_generation.get()
            || self.character != *fence.character_id.as_bytes()
            || self.session != fence.game_session_id
            || self.lease_generation != fence.character_lease_generation
            || !rule.recipient(self.position, self.dealt_damage)
            || (rule.top_damage_only && self.top_damage_character != Some(self.character))
        {
            return Ok(false);
        }
        // The root and quest rows are already locked. A mission accepted/reset after
        // the lethal snapshot cannot consume that earlier death, even if its value matches.
        for effect in &rule.effects {
            if rule
                .epoch_tracks
                .as_ref()
                .is_some_and(|guards| !guards.contains(&effect.track))
            {
                continue;
            }
            let changed_later: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND committed_character_revision > $3::text::numeric)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&effect.track)
                .bind(death.observed_revision.to_string()).fetch_one(&mut **tx).await?;
            if changed_later {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, WorldId};

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    #[test]
    fn source_death_receipt_keeps_legacy_binding_without_top_damage_proof()
    -> std::result::Result<(), String> {
        let mut source = SourceDeathContext {
            scope: RuntimeScopeRefV1::Channel {
                world_id: WorldId::decode(&id(80)).map_err(|error| format!("{error:?}"))?,
                channel_id: ChannelId::decode(&id(81)).map_err(|error| format!("{error:?}"))?,
            },
            scope_generation: 7,
            character: id(83),
            session: GameSessionId::decode(&id(82)).map_err(|error| format!("{error:?}"))?,
            lease_generation: 9,
            position: [33116, 32408, -7],
            dealt_damage: true,
            top_damage_character: None,
        };
        let mut legacy = [0xa5; 33];
        source.bind(&mut legacy);
        // Fixed digest from the pre-Herald source-quest-death-v1 byte layout.
        assert_eq!(legacy[0], 0xa5);
        assert_eq!(
            legacy[1..],
            [
                231, 209, 29, 255, 142, 178, 9, 0, 250, 183, 13, 252, 70, 52, 230, 124, 95, 119,
                207, 81, 24, 237, 25, 168, 2, 210, 227, 146, 201, 74, 90, 151,
            ]
        );
        source.top_damage_character = Some(source.character);
        let mut winner = [0xa5; 33];
        source.bind(&mut winner);
        assert_ne!(winner, legacy);
        source.top_damage_character = Some(id(84));
        let mut other = [0xa5; 33];
        source.bind(&mut other);
        assert_ne!(other, winner);
        Ok(())
    }
}
