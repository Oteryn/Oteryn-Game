//! Crystal src/lua/functions/creatures/creature_functions.cpp:591-608 and
//! src/game/game.cpp:8142-8155: item HP loss uses shields.
use super::*;
use crate::spell::combat_batch::ManaShieldState;

#[test]
#[allow(clippy::expect_used, reason = "fixed valid native player fixture")]
fn srvgap_c4_fireworks_respect_equipped_and_buffered_mana_shields() {
    let mut state = PlayerSpellState::new(
        crate::spell::cast::CharacterCastFacts {
            vocation: crate::spell::Vocation::Knight,
            level: 100,
            magic_level: 10,
            max_health: 1000,
            max_mana: 101,
            max_soul: 100,
        },
        0,
        0,
    )
    .expect("player");
    state.install_equipped_mana_shield(true);
    let before = state.clone();
    let (next, _) = stage_item_health_loss(&state, 10, 0).expect("firework");
    assert_eq!((next.health, next.mana), (1000, 81));
    assert_eq!(next.revision(), before.revision() + 1);
    assert_eq!(state, before);
    apply_combat_change(
        &mut state,
        &OwnerCombatChange::ManaShield(ManaShieldState {
            capacity: 3,
            expires_ms: 1000,
        }),
        0,
    )
    .expect("shield");
    let (next, _) = stage_item_health_loss(&state, 10, 0).expect("buffered firework");
    assert_eq!((next.health, next.mana), (993, 98));
}
