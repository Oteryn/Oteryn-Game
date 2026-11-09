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

#[test]
fn m51_reset_creature_census_uses_all_ten_exact_source_boxes() -> Result<(), String> {
    for pit in 1..=10 {
        let layout = Pit::new(pit).ok_or("pit")?.layout();
        let cells: Vec<_> = layout.reset_cells().collect();
        assert_eq!(cells.len(), 100);
        assert_eq!(cells.first(), Some(&layout.reset_from));
        assert_eq!(cells.last(), Some(&layout.reset_to));
        assert!(
            !layout.contains_reset(layout.pillar),
            "pillar lies outside the item/monster reset box"
        );
        let observed = cells.iter().copied().enumerate().collect::<Vec<_>>();
        assert_eq!(layout.monsters_to_reset(observed), Some((0..100).collect()));
        let mut outside = layout.reset_from;
        outside[0] -= 1;
        let mut other_floor = layout.center;
        other_floor[2] -= 1;
        assert_eq!(
            layout.monsters_to_reset([(1, layout.center), (2, outside), (3, other_floor)]),
            Some(vec![1])
        );
    }
    Ok(())
}

#[test]
fn m51_reset_creature_census_refuses_ambiguous_cells_and_actor_aliases() -> Result<(), String> {
    let layout = Pit::new(1).ok_or("pit")?.layout();
    assert!(
        layout
            .monsters_to_reset([(1, layout.center), (2, layout.center)])
            .is_none()
    );
    assert!(
        layout
            .monsters_to_reset([(1, layout.center), (1, layout.summon)])
            .is_none()
    );
    assert_eq!(layout.monsters_to_reset::<u64>([]), Some(Vec::new()));
    Ok(())
}

#[test]
fn m51_reset_items_selects_only_the_original_top_thing_and_preserves_source_exclusions()
-> Result<(), String> {
    use arena::ResetTopThing;
    let pit = Pit::new(1).ok_or("pit")?.layout();
    let at = pit.reset_from;
    let selected = |thing| pit.item_to_reset(at, thing);
    assert_eq!(
        selected(ResetTopThing::Item {
            identity: 1,
            appearance: 3031,
            movable: true
        }),
        Some(1)
    );
    assert_eq!(
        selected(ResetTopThing::Item {
            identity: 2,
            appearance: 3031,
            movable: false
        }),
        None
    );
    assert_eq!(selected(ResetTopThing::<u64>::Creature), None);
    assert_eq!(selected(ResetTopThing::<u64>::Empty), None);
    for appearance in [21464, 1841, 2181, 2182, 2183] {
        assert_eq!(
            selected(ResetTopThing::Item {
                identity: 3,
                appearance,
                movable: true
            }),
            None
        );
    }
    let item = ResetTopThing::Item {
        identity: 4,
        appearance: 3031,
        movable: true,
    };
    assert_eq!(pit.item_to_reset(pit.reset_to, item), Some(4));
    assert_eq!(pit.item_to_reset([at[0] - 1, at[1], at[2]], item), None);
    assert_eq!(pit.item_to_reset([at[0], at[1], at[2] - 1], item), None);
    assert_eq!(pit.item_to_reset(pit.pillar, item), None);
    Ok(())
}
