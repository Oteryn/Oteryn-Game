//! Admission policy shared by Crystal 201189f8's two staged levers.
//! See actions/bosses_levers/{magma_bubble,timira_the_many-headed}.lua:2-11,
//! data/libs/functions/boss_lever.lua:63-70 and config.lua.dist:658-659.
use crate::durability::activity_handoff::boss_room::BossRoomDefinition;

pub(super) fn matches_source_policy(definition: &BossRoomDefinition) -> bool {
    const MAGMA: &str = "oteryn:creature.magma_bubble";
    const TIMIRA: &str = "oteryn:creature.timira_the_many_headed";
    let expected_creature = match definition.boss_key.as_str() {
        "oteryn:boss.magma_bubble" => MAGMA,
        "oteryn:boss.timira_the_many_headed" => TIMIRA,
        _ => return !matches!(definition.boss_creature_key.as_str(), MAGMA | TIMIRA),
    };
    let policy = &definition.policy;
    definition.boss_creature_key == expected_creature
        && policy.minimum_level == 250
        && policy.minimum_players == 1
        && policy.maximum_players == 5
        && definition.lever_positions.len() == 5
        && policy.cooldown_ms == 72_000_000
        && policy.time_limit_ms == 1_200_000
        && definition.after_kill_ms == 60_000
}
