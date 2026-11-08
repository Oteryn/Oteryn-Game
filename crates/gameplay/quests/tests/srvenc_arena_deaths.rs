//! Crystal svargrond_arena/creaturescripts_arena_kill.lua:2-38 and
//! data-global/lib/quests/svargrond_arena.lua:25-115,118-172.
use oteryn_gameplay_quests::quest::barbarian_arena::{self as arena, Pit, Tier};

#[test]
fn m51_source_death_requires_most_damage_credit_and_same_tier() -> Result<(), String> {
    let greenhorn = Tier::Greenhorn;
    let pit = Pit::new(7).ok_or("literal pit")?;
    assert!(arena::death_plan(greenhorn, pit, "oteryn:creature.frostfur", false, true).is_none());
    assert!(arena::death_plan(greenhorn, pit, "oteryn:creature.avalanche", true, true).is_none());
    let plan = arena::death_plan(greenhorn, pit, "oteryn:creature.frostfur", true, true)
        .ok_or("source tier membership, not an invented exact-pit creature guard")?;
    assert_eq!(plan.next_pit, 8);
    let opening = plan.opening.ok_or("existing pillar opens passage")?;
    assert_eq!(
        (
            opening.remove_item,
            opening.portal_item,
            opening.portal_action
        ),
        (1841, 5022, 25200)
    );
    assert_eq!(opening.position, pit.layout().pillar);
    Ok(())
}

#[test]
fn m51_missing_pillar_still_advances_source_progress_without_minting_a_portal() -> Result<(), String>
{
    let plan = arena::death_plan(
        Tier::Warlord,
        Pit::new(10).ok_or("last pit")?,
        "oteryn:creature.the_obliverator",
        true,
        false,
    )
    .ok_or("credited source kill")?;
    assert_eq!(plan.next_pit, 11);
    assert!(plan.opening.is_none());
    assert_eq!(
        plan.speech,
        "Victory! Head through the new teleporter into the next room."
    );
    Ok(())
}

#[test]
fn m51_last_pit_preserves_the_source_asymmetric_geometry_and_occupancy_readset()
-> Result<(), String> {
    let pit = Pit::new(10).ok_or("last pit")?.layout();
    assert_eq!(pit.reset_from, [32184, 31052, -7]);
    assert_eq!(pit.reset_to, [32193, 31061, -7]);
    assert_eq!(pit.center, [32189, 31057, -7]);
    assert_eq!(pit.pillar, [32183, 31057, -7]);
    assert_eq!(pit.summon, [32189, 31059, -7]);
    assert!(pit.contains_occupant([32194, 31062, -7]));
    assert!(!pit.contains_occupant([32194, 31062, -8]));
    assert!(!pit.contains_occupant([32195, 31062, -7]));
    assert!(Pit::new(0).is_none());
    assert!(Pit::new(11).is_none());
    assert_eq!(arena::KICK_AFTER_SECONDS, 600);
    Ok(())
}

#[test]
fn m51_unregistered_player_on_source_spectator_edge_blocks_entry() -> Result<(), String> {
    let pit = Pit::new(10).ok_or("last pit")?.layout();
    assert!(pit.occupied_by_other(&1_u64, [(1, pit.center), (2, [32194, 31062, -7])]));
    assert!(!pit.occupied_by_other(&1_u64, [(1, pit.center), (2, [32195, 31062, -7])]));
    assert!(!pit.occupied_by_other(&1_u64, [(1, pit.center), (2, [32194, 31062, -8])]));
    assert!(!pit.occupied_by_other(&1_u64, [(1, pit.center)]));
    Ok(())
}
