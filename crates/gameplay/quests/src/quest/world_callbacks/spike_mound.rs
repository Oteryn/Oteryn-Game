//! Documented Spirit Shovel outcomes for an exact Ominous Mound.
use super::{Callback, Trigger};
use std::ops::RangeInclusive;

pub fn chances(callback: &Callback) -> Option<RangeInclusive<u8>> {
    let position = callback.selector.position?;
    if !(-10..=-8).contains(&position.floor) {
        return None;
    }
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
pub(super) const MAX_PUBLISHED: usize = 29 * 5 + 29 + 2;

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
    fn spike_reports_settle_after_five_nests_or_any_four_distinct_outposts() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let callbacks = crate::quest::reward_callbacks::load_for(catalogue)?;
        let mut sites = Vec::new();
        for row in &callbacks {
            if row.npc_area.is_some() {
                sites.push(row.key.as_str());
            }
        }
        assert_eq!(sites.len(), 5);
        let apply = |key: &str,
                     tracks: &mut std::collections::BTreeMap<String, i64>|
         -> Result<(), String> {
            let transition = catalogue
                .transition(key)
                .ok_or_else(|| format!("missing {key}"))?;
            for change in catalogue
                .evaluate(transition, tracks, 100_000)
                .map_err(|error| format!("{key}: {error:?}"))?
            {
                tracks.insert(change.track, change.after);
            }
            Ok(())
        };
        for omitted in &sites {
            let mut tracks = std::collections::BTreeMap::new();
            apply("oteryn:quest-reward/l12-spike-parcel-accept", &mut tracks)?;
            for site in sites.iter().filter(|site| *site != omitted) {
                assert!(apply("oteryn:quest-reward/l12-spike-parcel-report", &mut tracks).is_err());
                apply(site, &mut tracks)?;
                assert!(apply(site, &mut tracks).is_err());
            }
            apply("oteryn:quest-reward/l12-spike-parcel-report", &mut tracks)?;
        }

        let mut tracks = std::collections::BTreeMap::new();
        apply("oteryn:quest-reward/l12-spike-nests-accept", &mut tracks)?;
        apply(
            "oteryn:quest-callback/spike/transform-32242-32610-11",
            &mut tracks,
        )?;
        let nest = "oteryn:quest-callback/spike/nest-32162-32507-11";
        for _ in 0..5 {
            assert!(apply("oteryn:quest-reward/l12-spike-nests-report", &mut tracks).is_err());
            apply(nest, &mut tracks)?;
        }
        assert!(apply(nest, &mut tracks).is_err());
        apply("oteryn:quest-reward/l12-spike-nests-report", &mut tracks)?;
        Ok(())
    }

    #[test]
    fn each_mound_draw_has_one_result_and_only_the_soul_settles_release() -> Result<(), String> {
        let callbacks = crate::quest::world_callbacks::load()?;
        let mut mounds = std::collections::BTreeMap::new();
        let mut nests = 0;
        let mut pads = 0;
        for row in &callbacks {
            if let Some(range) = chances(row) {
                let p = row.selector.position.ok_or("mound position")?;
                let draws = mounds.entry((p.x, p.y, p.floor)).or_insert([0u8; 100]);
                for draw in range {
                    draws[usize::from(draw - 1)] += 1;
                    let phase = row
                        .storage
                        .iter()
                        .find(|s| s.track.ends_with("/release/progress"))
                        .ok_or("release phase")?;
                    assert_eq!(phase.set == 2, draw >= 90);
                    assert_eq!(
                        row.item_on_target.as_ref().ok_or("spirit shovel")?.reusable,
                        draw < 90
                    );
                }
            } else if row.key.starts_with("oteryn:quest-callback/spike/nest-") {
                assert_eq!(row.lever, Some([19209, 19210]));
                assert_eq!(row.effects.reset_after_ms, Some(120000));
                assert!(row.storage[0].increment && row.storage[0].maximum == 5);
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
                assert!(row.return_on_failure);
                assert!(matches!(
                    row.state_ops[0].effect,
                    crate::quest::reward_callbacks::StateMutation::SetNow
                ));
                pads += 1;
            }
        }
        assert_eq!((mounds.len(), nests, pads), (29, 29, 2));
        assert!(mounds.contains_key(&(32067, 32715, -10)));
        assert!(mounds.values().all(|draws| *draws == [1; 100]));
        Ok(())
    }
}
