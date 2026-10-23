//! Party spell feedback uses the existing private CHAT-2 egress and capability gating.
use super::{
    ComposedFreshAdmission,
    actor_spell::{SpellCastIntent, SpellCastOutcome},
};
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn party_spell_feedback(
        &self,
        session: GameSessionId,
        intent: &SpellCastIntent,
        outcome: &SpellCastOutcome,
    ) {
        let Some((spell, _)) = self.spells.source_indexed(intent.spell) else {
            return;
        };
        if !matches!(spell.execution, crate::spell::Execution::PartyBuff(_)) {
            return;
        }
        let text = match outcome.disposition {
            SpellCastDisposition::Cast => "Party spell cast.",
            SpellCastDisposition::CoolingDown => "Party spell is cooling down.",
            SpellCastDisposition::NotEnoughMana => "Not enough mana for the party spell.",
            SpellCastDisposition::NotEnoughSoul => "Not enough soul for the party spell.",
            SpellCastDisposition::LevelTooLow
            | SpellCastDisposition::MagicLevelTooLow
            | SpellCastDisposition::NotAvailable => "Party spell is not available.",
            SpellCastDisposition::TargetRequired
            | SpellCastDisposition::TargetIllegal
            | SpellCastDisposition::Rejected => "Party spell could not be cast.",
        };
        self.states.chat.push_party_spell_feedback(session, text);
    }
}
