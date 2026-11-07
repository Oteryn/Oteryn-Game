//! Closed Ajax source condition; the caller owns current NPC/session and receipt authority.
use super::*;

/// Crystal201189f data-global/npc/ajax.lua:149-152. Ten delayed ten-HP
/// FIRE ticks at two-second intervals; this stages no immediate HP mutation.
pub fn stage_ajax_source_fire(
    state: &PlayerSpellState,
    now_us: u64,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    if state.health == 0 {
        return rejected();
    }
    let mut next = state.clone();
    let fire = definition(
        "npc.ajax.source_fire",
        ConditionValues::DamageOverTime {
            element: oteryn_gameplay_domain::domain::condition::DotElement::Fire,
            total_min: 100,
            total_max: 100,
            per_tick: 10,
            interval_ms: 2000,
            delayed: true,
        },
    )?;
    // Preserve the actual owner's microsecond observation. Rounding backwards
    // would reject a store just advanced by another native condition owner.
    let decision = GameplayDecisionRoot::from_bytes([0; 32]);
    let facts = ApplicationFacts {
        now: now_us,
        base_speed: u16::try_from(state.base_speed).map_err(|_| SpellCastDisposition::Rejected)?,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &decision,
        occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
    };
    match next
        .conditions
        .apply(&fire, None, ConditionSourceKind::SelfUse, &[], &facts)
    {
        Ok(_) | Err(oteryn_gameplay_domain::domain::condition::ConditionRefusal::KeptCurrent) => {}
        Err(_) => return rejected(),
    }
    next.advance_batch_revision()?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_ajax_source_fire_is_ten_delayed_two_second_ticks_and_preserves_before()
    -> Result<(), Box<dyn std::error::Error>> {
        let before = PlayerSpellState::new(
            super::super::super::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::Knight,
                level: 100,
                magic_level: 10,
                max_health: 1000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .ok_or("valid native source actor")?;
        let mut after = stage_ajax_source_fire(&before, 0).map_err(|e| format!("{e:?}"))?;
        assert_eq!(after.health, before.health);
        assert_eq!(after.mana, before.mana);
        assert_eq!(after.revision(), before.revision() + 1);
        assert!(!has_periodic_damage(&before));
        assert!(
            after
                .conditions
                .take_due(1_999_999, TickFacts::default())
                .is_empty()
        );
        for tick in 1_u64..=10 {
            let due = after
                .conditions
                .take_due(tick * 2_000_000, TickFacts::default());
            assert_eq!(due.len(), 1);
            assert!(matches!(
                due[0].kind,
                TickKind::Damage {
                    element: oteryn_gameplay_domain::domain::condition::DotElement::Fire,
                    amount: 10,
                    refused: false
                }
            ));
        }
        assert!(
            after
                .conditions
                .take_due(22_000_000, TickFacts::default())
                .is_empty()
        );
        assert!(!has_periodic_damage(&after));
        let mut advanced = before.clone();
        advanced.conditions.take_due(1001, TickFacts::default());
        let mut exact = stage_ajax_source_fire(&advanced, 1002).map_err(|e| format!("{e:?}"))?;
        assert!(
            exact
                .conditions
                .take_due(2_001_001, TickFacts::default())
                .is_empty()
        );
        assert_eq!(
            exact
                .conditions
                .take_due(2_001_002, TickFacts::default())
                .len(),
            1
        );
        let mut dead = before.clone();
        dead.health = 0;
        assert!(stage_ajax_source_fire(&dead, 0).is_err());
        Ok(())
    }
}
