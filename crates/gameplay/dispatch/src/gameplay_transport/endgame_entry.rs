//! Endgame prerequisites for the shared qualified native activity owner.
//! Crystal 201189f actions/bosses_levers/the_primal_manace.lua:6-12;
//! quests/rotten_blood_quest/movements_blood_entrance.lua:20-32.

use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::character_progression::CharacterProgressionError;
use oteryn_gameplay_encounters::bosses::endgame_entry::{
    self, Encounter, EntryFacts, EntryRefusal, QuestFamily,
};
use oteryn_protocol_oteryn::boss_encounter as wire;

/// Crystal 201189f the_primal_manace.lua:2-4 binds the lever boss to its
/// captured creature. Template and creature aliases must share entry policy.
pub(super) fn canonical_boss_key(key: &str) -> Option<String> {
    // Soul War's source rooms are declared in soul_war.lua rather than the
    // literal lever capture. Reuse the same closed phase identity as entry;
    // the activity owner still requires its independently qualified room.
    let creature = key
        .strip_prefix("oteryn:boss.")
        .map(|suffix| format!("oteryn:creature.{suffix}"));
    if let Some(encounter) = Encounter::from_boss_key(creature.as_deref().unwrap_or(key)) {
        return encounter
            .boss_key()
            .strip_prefix("oteryn:creature.")
            .map(|suffix| format!("oteryn:boss.{suffix}"));
    }
    let candidates = crate::content::bosses::activity_template::captured_boss_levers().ok()?;
    candidates
        .iter()
        .find(|candidate| {
            candidate.creature_key.as_deref() == Some(key)
                || candidate.boss_key().as_deref() == Some(key)
        })?
        .boss_key()
}

pub(super) fn encounter_for_boss_key(key: &str) -> Option<Encounter> {
    let creature = key
        .strip_prefix("oteryn:boss.")
        .map(|suffix| format!("oteryn:creature.{suffix}"));
    if let Some(encounter) = Encounter::from_boss_key(creature.as_deref().unwrap_or(key)) {
        return Some(encounter);
    }
    let candidates = crate::content::bosses::activity_template::captured_boss_levers().ok()?;
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.boss_key().as_deref() == Some(key))?;
    Encounter::from_boss_key(candidate.creature_key.as_deref()?)
}

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
        let Some(encounter) = encounter_for_boss_key(&request.boss_key) else {
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

#[cfg(test)]
mod tests {
    #[test]
    fn canonical_endgame_aliases_retain_their_entry_policy() -> Result<(), String> {
        for key in [
            "oteryn:creature.goshnar_s_spite",
            "oteryn:creature.goshnar_s_megalomania_green",
            "oteryn:creature.the_primal_menace",
            "oteryn:creature.bakragore",
        ] {
            let canonical = super::canonical_boss_key(key).ok_or("canonical boss")?;
            assert_eq!(
                super::encounter_for_boss_key(&canonical),
                super::Encounter::from_boss_key(key),
                "canonical aliases must not bypass the prerequisite gate: {key}"
            );
        }
        Ok(())
    }
}
