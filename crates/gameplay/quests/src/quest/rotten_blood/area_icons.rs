//! Crystal201189f monster_onthink_area_icons.lua120-174 persistent player KV.
use crate::quest::QuestStateCatalogue;
pub const QUEST: &str = "oteryn:quest/rotten-blood/area-icons";
pub const PREFIX: &str = "oteryn:quest-transition/rotten-blood/area-icons/";
pub const TRACKS: [&str; 3] = [
    "oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update",
    "oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update",
    "oteryn:quest-progress/rotten-blood/area-icons/last-lower-left-update",
];
pub const SECONDS: [i64; 3] = [8, 30, 1];

pub const MINIONS: [&str; 15] = [
    "oteryn:creature.bloated_man_maggot",
    "oteryn:creature.converter",
    "oteryn:creature.darklight_construct",
    "oteryn:creature.darklight_emitter",
    "oteryn:creature.darklight_matter",
    "oteryn:creature.darklight_source",
    "oteryn:creature.darklight_striker",
    "oteryn:creature.meandering_mushroom",
    "oteryn:creature.mycobiontic_beetle",
    "oteryn:creature.oozing_carcass",
    "oteryn:creature.rotten_man_maggot",
    "oteryn:creature.sopping_carcass",
    "oteryn:creature.sopping_corpus",
    "oteryn:creature.walking_pillar",
    "oteryn:creature.wandering_pillar",
];

pub fn in_upper_left_source_cell(x: i32, y: i32, floor: i16) -> bool {
    (14..=15).contains(&floor) && (33800..=33941).contains(&x) && (31653..=31771).contains(&y)
}
pub fn in_lower_left_source_cell(x: i32, y: i32, floor: i16) -> bool {
    (14..=15).contains(&floor) && (33800..=33937).contains(&x) && (31809..=31931).contains(&y)
}

pub fn transition(mask: u8) -> Option<String> {
    (1..=7).contains(&mask).then(|| format!("{PREFIX}{mask}"))
}
pub(crate) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    use crate::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition,
    };
    let mut tracks = catalogue.tracks.into_values().collect::<Vec<_>>();
    let mut transitions = catalogue.transitions.into_values().collect::<Vec<_>>();
    for key in TRACKS {
        if tracks.iter().any(|track| track.key == key) {
            return Err(format!("duplicate Rotten area clock {key}"));
        }
        tracks.push(QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: i64::MAX,
        });
    }
    for mask in 1..=7_u8 {
        let key = transition(mask).ok_or("closed source clock mask")?;
        if transitions.iter().any(|transition| transition.key == key) {
            return Err(format!("duplicate Rotten area source transition {key}"));
        }
        transitions.push(QuestTransition {
            world_change: None,
            key,
            quest: QUEST.into(),
            effects: TRACKS
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(index, track)| QuestEffect {
                    track: (*track).into(),
                    from: QuestComparison::ElapsedAtLeast(SECONDS[index]),
                    effect: QuestEffectKind::SetNow,
                })
                .collect(),
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestComparison, QuestEffectKind};
    #[test]
    fn srx3_rotten_area_three_persistent_kv_clocks_use_exact_database_time_thresholds()
    -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        for track in TRACKS {
            let actual = catalogue
                .track(track)
                .ok_or_else(|| format!("missing persistent donor KV {track}"))?;
            assert_eq!(actual.quest, QUEST);
        }
        for mask in 1..=7 {
            let key = transition(mask).ok_or("closed due mask")?;
            let actual = catalogue
                .transition(&key)
                .ok_or_else(|| format!("missing exact source due mask {mask}"))?;
            assert_eq!(actual.quest, QUEST);
            assert_eq!(actual.effects.len(), mask.count_ones() as usize);
            for (index, track) in TRACKS.into_iter().enumerate() {
                let actual = actual.effects.iter().find(|effect| effect.track == track);
                if mask & (1 << index) == 0 {
                    assert!(actual.is_none());
                    continue;
                }
                let actual = actual.ok_or("missing due timestamp")?;
                assert_eq!(actual.from, QuestComparison::ElapsedAtLeast(SECONDS[index]));
                assert_eq!(actual.effect, QuestEffectKind::SetNow);
            }
        }
        assert!(transition(0).is_none());
        assert!(transition(8).is_none());
        Ok(())
    }
}
