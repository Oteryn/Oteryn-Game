//! Source XP uses the existing bounded, receipt-bound quest obligation owner.
use super::*;
use oteryn_gameplay_quests::quest::{QUESTGATE0_RL_05, npc_rules::NpcRuleEffect};

pub(super) fn has_experience(effects: &[NpcRuleEffect]) -> bool {
    effects.iter().any(|effect| matches!(effect, NpcRuleEffect::Experience { .. }))
}

pub(super) fn experience(effects: &[NpcRuleEffect]) -> StoredResult<i64> {
    effects.iter().try_fold(0_i64, |total, effect| {
        let NpcRuleEffect::Experience { amount } = effect else { return Ok(total); };
        let amount = i64::try_from(*amount).map_err(|_| DurabilityError::InvalidStoredState)?;
        let total = total.checked_add(amount).ok_or(DurabilityError::InvalidStoredState)?;
        if amount <= 0 || total > QUESTGATE0_RL_05 { return Err(DurabilityError::InvalidStoredState); }
        Ok(total)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_xp_is_summed_once_within_the_shared_obligation_bound() -> std::result::Result<(), DurabilityError> {
        assert_eq!(experience(&[])?, 0);
        assert_eq!(experience(&[NpcRuleEffect::Experience { amount: 7 }, NpcRuleEffect::Experience { amount: 11 }])?, 18);
        assert_eq!(experience(&[NpcRuleEffect::Experience { amount: 100_000_000 }])?, QUESTGATE0_RL_05);
        for amounts in [[0, 1], [100_000_000, 1], [u64::MAX, 1]] {
            assert!(experience(&amounts.map(|amount| NpcRuleEffect::Experience { amount })).is_err());
        }
        Ok(())
    }
}
