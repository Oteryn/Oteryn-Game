//! Twenty Miles wiki69583@1149608 lines145/153/158/160/162: obtain Darkness,
//! Life and Light, then submit their energies to Guardian of the Deep Seas (NPC).
//! Inputs: one Mind Stone, one Seacrest Pearl, Bottle of Gloo (NOT consumed).
//! Geometry and receipt-backed orb entitlement are explicit Oteryn native choices;
//! no donor position, boss death or quest item reward quantity is invented.
use super::{
    QuestTrackChange,
    world_callbacks::{Callback, ItemOnTarget, Position, Selector, Storage, Trigger},
};
pub const QUEST: &str = "oteryn:quest.fire_feathered_native";
pub const COMPLETE: &str = "oteryn:quest-transition/fire-feathered/native-complete";
pub const TRACKS: [&str; 3] = [
    "oteryn:quest-progress/fire-feathered-native/darkness",
    "oteryn:quest-progress/fire-feathered-native/life",
    "oteryn:quest-progress/fire-feathered-native/light",
];
type OrbSource = (&'static str, (u16, u16, i8), u16, u16, bool);
pub const SOURCES: [OrbSource; 3] = [
    (
        "oteryn:quest-callback/fire-feathered-native-darkness",
        (33552, 31623, -14),
        21804,
        3062,
        true,
    ),
    (
        "oteryn:quest-callback/fire-feathered-native-life",
        (33553, 31623, -14),
        21812,
        21747,
        true,
    ),
    (
        "oteryn:quest-callback/fire-feathered-native-light",
        (33554, 31623, -14),
        21805,
        21803,
        false,
    ),
];
pub const GUARDIAN_NATIVE_POSITION: (u16, u16, i8) = (33553, 31624, -14);
pub fn callbacks() -> Vec<Callback> {
    SOURCES.iter().enumerate().map(|(index,(key,pos,appearance,input,consume))|Callback{admission: None,actor_kind: Default::default(), opening_only: false, requires_world_admission: false, facing: None, state_ops: Vec::new(), world_requirements: Vec::new(),
        key:(*key).into(),quest:QUEST.into(),trigger:Trigger::Use,
        selector:Selector{native: None,position:Some(Position{x:pos.0,y:pos.1,floor:pos.2}),unique:None,action:None,appearance:Some(*appearance)},
        lever:None,reversible:false,passages:vec![],teleport:None,
        storage:vec![Storage{increment: false, at_least: false,track:TRACKS[index].into(),minimum:0,maximum:0,set:1,preserve:false}],
        minimum_level:None,experience: None, world_change: None,monk_reward:None,offerings:vec![],
        item_on_target:Some(ItemOnTarget{selected_instance:false,alternatives: Vec::new(), reusable: false,item:format!("oteryn:item.tibia.i{input}"),count:1,replacement:(!consume).then(||format!("oteryn:item.tibia.i{input}")),target_replacement:None}),
        return_on_failure:false,failure_message:None,success_message:None,native_rule:None,effects:Default::default(),
        sources:vec!["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/guardian_of_the_deep_seas.lua:explicit-native-world-quest-adaptation;wiki69583@1149608:145,153,158,160,162;NOT_DONOR_GEOMETRY".into()],
    }).collect()
}
pub fn completed(changes: &[QuestTrackChange]) -> bool {
    changes.len() == 3
        && TRACKS.iter().all(|track| {
            changes
                .iter()
                .filter(|change| change.track == *track && (change.before, change.after) == (1, 0))
                .count()
                == 1
        })
}
pub fn extend(catalogue: super::QuestStateCatalogue) -> Result<super::QuestStateCatalogue, String> {
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};
    let mut tracks = catalogue.tracks.clone();
    let mut transitions = catalogue.transitions.clone();
    for track in TRACKS {
        tracks.insert(
            track.into(),
            QuestTrack {
                key: track.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            },
        );
    }
    transitions.insert(
        COMPLETE.into(),
        QuestTransition {
            key: COMPLETE.into(),
            quest: QUEST.into(),
            effects: TRACKS
                .into_iter()
                .map(|track| QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Between(1, 1),
                    effect: QuestEffectKind::Set(0),
                })
                .collect(),
            completes: false,
            experience: None,
            world_change: None,
        },
    );
    super::QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worldchg_fire_feathered_requires_three_real_orb_receipts_and_preserves_gloo()
    -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        let transition = catalogue
            .transition(COMPLETE)
            .ok_or("native Guardian receipt missing")?;
        let mut stored = std::collections::BTreeMap::new();
        for callback in callbacks() {
            assert!(catalogue.evaluate(transition, &stored, 0).is_err());
            let source = catalogue
                .transition(&callback.key)
                .ok_or("native orb callback missing")?;
            let changes = catalogue
                .evaluate(source, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            stored.insert(changes[0].track.clone(), changes[0].after);
            assert!(catalogue.evaluate(source, &stored, 0).is_err());
        }
        assert!(completed(
            &catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?
        ));
        let light = callbacks()
            .pop()
            .ok_or("light")?
            .item_on_target
            .ok_or("gloo")?;
        assert_eq!(light.item, "oteryn:item.tibia.i21803");
        assert_eq!(light.replacement, Some(light.item));
        Ok(())
    }
}
