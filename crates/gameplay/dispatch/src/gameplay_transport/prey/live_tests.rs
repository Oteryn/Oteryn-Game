use super::*;
use crate::gameplay_transport::actor_spell::tests as fixture;

fn slot(number: u8, creature: &str, bonus: u8) -> wire::Slot {
    wire::Slot {
        slot: number, state: 2, creatures: vec![creature.to_owned()],
        selected: creature.to_owned(), bonus, grade: 2, percent: 5,
        hunting_seconds: 7200, option: 0, free_reroll_at: 0,
        chosen_tier: 1, allowed_tier: 5, selection_epoch: 1,
    }
}

fn live(bonus: u8, mode: PreyDefenseMode) -> LivePrey {
    let (_, actor, session) = fixture::runtime_with_player(139);
    LivePrey {
        actor, session,
        reply: wire::Reply {
            status: 0, wildcard_balance: 0,
            slots: vec![slot(1, "oteryn:creature.rat", bonus), slot(2, "oteryn:creature.dragon", bonus)],
            list_size: 9, expansion_owned: false, offered_today: Vec::new(), offered_next: String::new(),
        },
        permanent: [true; 3], activity: [SlotActivity::default(); 3],
        defense_mode: mode, pending: None,
    }
}

fn event(creature: &str, kind: PreyActivityKind, second: u64) -> PreyActivity<'_> {
    PreyActivity { creature, kind, now: second * 1_000_000 }
}

#[test]
fn prey_diff_offensive_clocks_only_consume_for_damage_to_the_selected_race() {
    for bonus in [1, 3, 4] {
        let mut prey = live(bonus, PreyDefenseMode::DealtOrAttacked);
        for second in [0, 30, 60, 90, 120] {
            prey.record_activity(event("oteryn:creature.troll", PreyActivityKind::DealtDamage, second), true);
        }
        assert_eq!(prey.reply.slots[0].hunting_seconds, 7200);
        for second in [120, 150, 180] {
            prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, second), true);
        }
        assert_eq!(prey.reply.slots[0].hunting_seconds, 7140);
        assert_eq!(prey.reply.slots[1].hunting_seconds, 7200);
        assert_eq!(prey.checkpoints().len(), 1);
        assert_eq!(prey.checkpoints()[0].selection_epoch, 1);
    }
}

#[test]
fn prey_diff_two_selected_races_have_independent_clocks() {
    let mut prey = live(1, PreyDefenseMode::DealtOrAttacked);
    prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, 0), true);
    prey.record_activity(event("oteryn:creature.dragon", PreyActivityKind::DealtDamage, 30), true);
    prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, 60), true);
    assert_eq!(prey.reply.slots[0].hunting_seconds, 7140);
    assert_eq!(prey.reply.slots[1].hunting_seconds, 7200);
    prey.record_activity(event("oteryn:creature.dragon", PreyActivityKind::DealtDamage, 90), true);
    assert_eq!(prey.reply.slots[1].hunting_seconds, 7140);
    assert_eq!(prey.checkpoints().len(), 2);
}

#[test]
fn prey_diff_defense_switch_distinguishes_fully_blocked_or_absorbed_attacks() {
    for (mode, expected) in [(PreyDefenseMode::ReceivedDamage, 7200), (PreyDefenseMode::DealtOrAttacked, 7140)] {
        let mut prey = live(2, mode);
        for second in [0, 30, 60] {
            prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::Attacked, second), true);
        }
        assert_eq!(prey.reply.slots[0].hunting_seconds, expected);
        assert_eq!(prey.reply.slots[1].hunting_seconds, 7200);
    }
}

#[test]
fn prey_diff_v1_requires_received_damage_and_v2_also_accepts_outgoing_damage() {
    for (mode, expected) in [(PreyDefenseMode::ReceivedDamage, 7200), (PreyDefenseMode::DealtOrAttacked, 7140)] {
        let mut prey = live(2, mode);
        for second in [0, 30, 60] {
            prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, second), true);
        }
        assert_eq!(prey.reply.slots[0].hunting_seconds, expected);
    }
    for mode in [PreyDefenseMode::ReceivedDamage, PreyDefenseMode::DealtOrAttacked] {
        let mut prey = live(2, mode);
        for second in [0, 30, 60] {
            prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::ReceivedDamage, second), true);
        }
        assert_eq!(prey.reply.slots[0].hunting_seconds, 7140);
    }
}

#[test]
fn prey_diff_idle_stale_and_duplicate_events_never_catch_up() {
    let mut prey = live(1, PreyDefenseMode::DealtOrAttacked);
    for second in [0, 30, 30, 29, 3600] {
        prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, second), true);
    }
    assert_eq!(prey.reply.slots[0].hunting_seconds, 7200);
    prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, 3630), true);
    assert_eq!(prey.reply.slots[0].hunting_seconds, 7140);
}

#[test]
fn prey_diff_query_preserves_partial_minute_but_new_selection_resets_it() {
    let mut prey = live(1, PreyDefenseMode::DealtOrAttacked);
    prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, 0), true);
    prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, 30), true);
    prey.install(prey.reply.clone(), [true; 3], prey.defense_mode);
    prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, 60), true);
    assert_eq!(prey.reply.slots[0].hunting_seconds, 7140);
    let mut next = prey.reply.clone();
    next.slots[0].selection_epoch = 2;
    next.slots[0].hunting_seconds = 7200;
    prey.install(next, [true; 3], prey.defense_mode);
    for second in [90, 120] {
        prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, second), true);
    }
    assert_eq!(prey.reply.slots[0].hunting_seconds, 7200);
    prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, 150), true);
    assert_eq!(prey.reply.slots[0].hunting_seconds, 7140);
    assert_eq!(prey.checkpoints()[0].selection_epoch, 2);
}

#[test]
fn prey_diff_locked_premium_slot_has_no_activity_and_no_checkpoint() {
    let mut prey = live(1, PreyDefenseMode::DealtOrAttacked);
    prey.permanent[1] = false;
    for second in [0, 30, 60] {
        prey.record_activity(event("oteryn:creature.dragon", PreyActivityKind::DealtDamage, second), false);
    }
    assert_eq!(prey.reply.slots[1].hunting_seconds, 7200);
    assert!(prey.checkpoints().is_empty());
}

#[test]
fn prey_diff_expiry_causing_kill_is_measured_but_later_kills_are_not() {
    let mut prey = live(1, PreyDefenseMode::DealtOrAttacked);
    prey.reply.slots[0].hunting_seconds = 60;
    for second in [0, 30, 60] {
        prey.record_activity(event("oteryn:creature.rat", PreyActivityKind::DealtDamage, second), true);
    }
    assert_eq!(prey.reply.slots[0].hunting_seconds, 0);
    assert!(prey.kill_checkpoint("oteryn:creature.rat", 60_000_000, true).is_some());
    assert!(prey.kill_checkpoint("oteryn:creature.rat", 60_000_001, true).is_none());
    assert!(prey.kill_checkpoint("oteryn:creature.troll", 60_000_000, true).is_none());
}
