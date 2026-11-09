//! SRV6: literal level-999 island ingress, on the existing fenced STEP callback owner.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8
//! data-global/scripts/movements/teleport/schrodingers_island_teleport_lvl_999.lua:3-27.
//! The visit is retained by the existing QuestState receipt owner for Robinson Crusoe;
//! the source teleport and level requirement remain unchanged.
use super::world_callbacks::{Callback, Position, Selector, Storage, Trigger};

pub const QUEST: &str = "oteryn:quest.schrodingers_island_visit";
pub const VISITED: &str = "oteryn:quest-progress/schrodingers_island/visited";
pub const ENTER: &str = "oteryn:quest-callback/srv6-schrodinger-island";

pub(crate) fn callback() -> Callback {
    Callback { admission: None, requires_premium: false,
    Callback { required_completed_quests: Vec::new(), admission: None,
world_change: None,
        actor_kind: Default::default(), opening_only: false, requires_world_admission: false,
        key: ENTER.into(),
        quest: QUEST.into(),
        trigger: Trigger::StepIn,
        selector: Selector { native: None, position:None, unique:None, action:Some(15998), appearance:None },
        lever:None,
        reversible:false,
        passages:Vec::new(),
        teleport:Some(Position { x:32832, y:32435, floor:-7 }),
        facing:None,
        state_ops: Vec::new(), world_requirements: Vec::new(),
        storage:vec![Storage { track:VISITED.into(), minimum:-1, maximum:1, set:1, preserve:false, increment:false, at_least:false }],
        minimum_level:Some(999), character_requirements: None,
        experience:None,
        monk_reward:None,
        offerings:Vec::new(),
        item_on_target:None,
        return_on_failure:true,
        failure_message:Some("You need level 999 to enter here.".into()),
        success_message:Some("Congratulations!".into()),
        native_rule:None, effects:Default::default(),
        sources:vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/movements/teleport/schrodingers_island_teleport_lvl_999.lua:3-27".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv6_schrodinger_callback_preserves_source_level_action_and_destination() {
        let source = callback();
        assert_eq!(source.selector.action, Some(15998));
        assert_eq!(source.minimum_level, Some(999));
        assert_eq!(
            source.teleport,
            Some(Position {
                x: 32832,
                y: 32435,
                floor: -7
            })
        );
        assert!(source.return_on_failure);
        assert_eq!(source.storage[0].track, VISITED);
    }
}
