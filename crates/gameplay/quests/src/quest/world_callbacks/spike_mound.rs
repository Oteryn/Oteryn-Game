//! Documented Spirit Shovel outcomes for an exact Ominous Mound.
use super::{Callback, Trigger};
use std::ops::RangeInclusive;

pub fn chances(callback: &Callback) -> Option<RangeInclusive<u8>> {
    let position = callback.selector.position?;
    let suffix = callback
        .key
        .strip_prefix("oteryn:quest-callback/spike/mound-")?;
    let (site, result) = suffix.split_once('/')?;
    if site != format!("{}-{}-{}", position.x, position.y, -position.floor)
        || callback.quest != "oteryn:quest.spike_task"
        || callback.trigger != Trigger::Use
        || !callback.requires_world_admission
        || callback.selector.appearance != Some(19211)
        || callback.selector.ordinal.is_none()
        || callback.lever != Some([19211, 19212])
        || callback.reversible
    {
        return None;
    }
    let phase = callback
        .storage
        .iter()
        .find(|s| s.track == "oteryn:quest-progress/spike/release/progress")?;
    let tool = callback.item_on_target.as_ref()?;
    let soul = result == "enraged_soul";
    if (phase.minimum, phase.maximum, phase.set, phase.preserve)
        != (1, 1, if soul { 2 } else { 1 }, !soul)
        || tool.item != "oteryn:item.tibia.i19203"
        || !tool.selected_instance
        || tool.count != 1
        || tool.reusable == soul
        || tool.replacement.is_some()
        || tool.target_replacement.is_some()
        || tool.target_remove
        || !tool.ingredients.is_empty()
        || !tool.alternatives.is_empty()
    {
        return None;
    }
    if result == "empty" {
        return (callback.effects.encounter.is_none()
            && callback.effects.reset_after_ms == Some(120_000))
        .then_some(1..=49);
    }
    let encounter = callback.effects.encounter.as_ref()?;
    let [monster] = encounter.monsters.as_slice() else {
        return None;
    };
    if monster.position != position
        || monster.creature != format!("oteryn:creature.{result}")
        || !encounter.players.is_empty()
        || encounter.room_min != position
        || encounter.room_max != position
        || encounter.minimum_level != 25
        || encounter.phase.is_some()
        || !encounter
            .timing
            .is_some_and(|clock| clock.spawn_after_ms == 1 && clock.reset_after_ms == 120_000)
    {
        return None;
    }
    match result {
        "zombie" => Some(50..=69),
        "demon_skeleton" => Some(70..=79),
        "ghost" => Some(80..=89),
        "enraged_soul" => Some(90..=100),
        _ => None,
    }
}

pub(super) const DOCUMENT: &str =
    include_str!("../../../../../../content/quests/world-callbacks/spike-objectives.json");
pub(super) const MAX_PUBLISHED: usize = 28 * 5 + 29 + 2;

pub(super) fn published_count(callbacks: &[Callback]) -> Result<usize, String> {
    static ROWS: super::OnceLock<Result<super::BTreeMap<String, Callback>, String>> =
        super::OnceLock::new();
    let eligible = ROWS
        .get_or_init(|| {
            super::parse(DOCUMENT)
                .map(|rows| rows.into_iter().map(|c| (c.key.clone(), c)).collect())
        })
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(callbacks
        .iter()
        .filter(|c| eligible.get(&c.key) == Some(*c))
        .count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mound_draw_has_one_result_and_only_the_soul_settles_release() -> Result<(), String> {
        let callbacks = crate::quest::world_callbacks::load()?;
        let mut mounds = std::collections::BTreeMap::new();
        let mut nests = 0;
        let mut pads = 0;
        for row in &callbacks {
            if let Some(range) = chances(row) {
                let p = row.selector.position.unwrap();
                let draws = mounds.entry((p.x, p.y, p.floor)).or_insert([0u8; 100]);
                for draw in range {
                    draws[usize::from(draw - 1)] += 1;
                    let phase = row
                        .storage
                        .iter()
                        .find(|s| s.track.ends_with("/release/progress"))
                        .unwrap();
                    assert_eq!(phase.set == 2, draw >= 90);
                    assert_eq!(row.item_on_target.as_ref().unwrap().reusable, draw < 90);
                }
            } else if row.key.starts_with("oteryn:quest-callback/spike/nest-") {
                assert_eq!(row.lever, Some([19209, 19210]));
                assert_eq!(row.effects.reset_after_ms, Some(120000));
                assert!(row.storage[0].increment && row.storage[0].maximum == 8);
                assert!(matches!(
                    row.state_ops[0].from,
                    crate::quest::reward_callbacks::StateCondition::ElapsedLessThan {
                        seconds: 120
                    }
                ));
                nests += 1;
            } else if row
                .key
                .starts_with("oteryn:quest-callback/spike/transform-")
            {
                assert_eq!(row.trigger, Trigger::StepIn);
                assert!(matches!(
                    row.state_ops[0].effect,
                    crate::quest::reward_callbacks::StateMutation::SetNow
                ));
                pads += 1;
            }
        }
        assert_eq!((mounds.len(), nests, pads), (28, 29, 2));
        assert!(mounds.values().all(|draws| *draws == [1; 100]));
        Ok(())
    }
}
