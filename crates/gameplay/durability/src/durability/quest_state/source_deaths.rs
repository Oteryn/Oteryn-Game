//! Captured native source eligibility is evidence, never current mutation authority.
//! Crystal 201189f HeartBossDeath:26-138; Court Warlock:323-329;
//! MagmaBubbleDeath:12-23; GoblinLeaderDeath:3-8. Current fences remain independently checked.
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
