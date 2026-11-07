//! Endgame prerequisites for the shared qualified native activity owner.
//! Crystal 201189f actions/bosses_levers/the_primal_manace.lua:6-12;
//! quests/rotten_blood_quest/movements_blood_entrance.lua:20-32.

use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::character_progression::CharacterProgressionError;
use oteryn_gameplay_encounters::bosses::endgame_entry::{
    self, Encounter, EntryFacts, EntryRefusal, QuestFamily,
};
use oteryn_protocol_oteryn::boss_encounter as wire;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn endgame_boss_entry(
        &self,
        admitted: AdmittedSession,
        payload: &[u8],
    ) -> Result<Result<(), EntryRefusal>, CharacterProgressionError> {
        let request = match wire::decode_boss_key(payload) {
            Ok(request) => request,
            Err(_) => return Ok(Err(EntryRefusal::InvalidInput)),
        };
        let Some(encounter) = Encounter::from_boss_key(&request.boss_key) else {
            return Ok(Err(EntryRefusal::ContentUnavailable));
        };
        let Some(fence) =
            super::character_gameplay_fence::current(self, admitted.game_session_id, false).await
        else {
            return Err(CharacterProgressionError::AuthorityRejected);
        };
        if fence.connection_generation.get() != admitted.continuity.connection_generation
            || admitted.item_fence.is_none_or(|expected| {
                expected.character_id != fence.character_id
                    || expected.character_lease_generation != fence.character_lease_generation
                    || expected.runtime_scope != fence.runtime_scope
                    || expected.scope_ownership_generation != fence.scope_ownership_generation
            })
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let Some(progression) = self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await?
        else {
            return Err(CharacterProgressionError::MissingProgressionState);
        };
        if progression.character_revision != fence.expected_character_revision {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let (access, chain_ready) = match encounter.family() {
            QuestFamily::PrimalOrdeal => (true, true),
            QuestFamily::SoulWar => {
                let state = self
                    .root
                    .read_soul_war_state(self.character, self.holder, fence)
                    .await?;
                (state.access(), state.chain_ready())
            }
            QuestFamily::RottenBlood => {
                let state = self
                    .root
                    .read_rotten_blood_state(self.character, self.holder, fence)
                    .await?;
                let boss =
                    oteryn_gameplay_quests::quest::rotten_blood::RottenBloodBoss::from_creature_key(
                        encounter.boss_key(),
                    );
                if let Err(refusal) = state.validate_entry(progression.level, boss) {
                    use oteryn_gameplay_quests::quest::rotten_blood::RottenBloodRefusal;
                    return Ok(Err(match refusal {
                        RottenBloodRefusal::LevelTooLow => EntryRefusal::Level,
                        RottenBloodRefusal::AccessMissing => EntryRefusal::QuestAccess,
                        RottenBloodRefusal::ChainIncomplete => EntryRefusal::BossChain,
                        _ => EntryRefusal::InvalidInput,
                    }));
                }
                (state.access(), state.chain_ready())
            }
        };
        let facts = EntryFacts {
            character: *fence.character_id.as_bytes(),
            character_revision: fence.expected_character_revision.get(),
            level: progression.level,
            access,
            chain_ready,
        };
        // This validates quest prerequisites only. The caller must still resolve a
        // qualified content template and perform the shared native scope handoff.
        Ok(endgame_entry::enter(encounter, &[facts], |_| async { Ok(()) }).await)
    }
}
