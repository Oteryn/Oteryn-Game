//! Crystal 201189f actions_lions_rock.lua:13-79. Shared pagodas, personal trials.
use super::world_callbacks::{
    Callback, ItemOnTarget, NativeSelector, Passage, Position, Selector, Storage,
};
use super::{QuestRefusal, QuestStateCatalogue, QuestTrack};

pub const QUEST: &str = "oteryn:quest.tibia_tales";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u10_70/lions_rock/questline";
pub const FLAGS: [&str; 3] = [
    "oteryn:quest-progress/quest/u10_70/lions_rock/outer_sanctum/lions_strength",
    "oteryn:quest-progress/quest/u10_70/lions_rock/outer_sanctum/lions_beauty",
    "oteryn:quest-progress/quest/u10_70/lions_rock/outer_sanctum/lions_tears",
];
pub const TRACKS: [[&str; 2]; 3] = [
    [QUESTLINE, FLAGS[0]],
    [QUESTLINE, FLAGS[1]],
    [QUESTLINE, FLAGS[2]],
];
pub const KEYS: [&str; 3] = [
    "oteryn:quest-callback/lions-rock-strength-first",
    "oteryn:quest-callback/lions-rock-beauty",
    "oteryn:quest-callback/lions-rock-tears",
];
const TARGETS: [(u16, u16, u8, u16); 3] = [
    (33137, 32291, 1, 21390),
    (33138, 32369, 3, 2110),
    (33154, 32279, 2, 21391),
];
const PAGODAS: [(u16, u16); 3] = [(33134, 32289), (33136, 32369), (33156, 32279)];
const ITEMS: [u16; 3] = [9634, 21389, 21466];
const MESSAGES: [&str; 3] = [
    "You have sacrificed a cobra tongue at an ancient statue. The light in the small pyramid nearby begins to shine.",
    "You burnt a lion's mane flower. The light in the small pyramid nearby begins to shine.",
    "You have purified a sacret pedestal with holy water. You have now passed the last testto enter thge inner sanctum.",
];

pub fn binding(key: &str) -> Option<usize> {
    KEYS.iter().position(|candidate| *candidate == key)
}

pub(crate) fn callbacks() -> Vec<Callback> {
    (0..3).map(|index| {
        let (x,y,ordinal,appearance) = TARGETS[index];
        let (px,py) = PAGODAS[index];
        let mut callback = super::source_world_objects::uneasy_crystal();
        callback.key = KEYS[index].into();
        callback.quest = QUEST.into();
        callback.selector = Selector {
            native: Some(NativeSelector { position:Position{x,y,floor:-8},ordinal,appearance }),
            position: Some(Position{x,y,floor:-8}), unique:None,action:None,appearance:Some(appearance),
        };
        callback.effects = Default::default();
        callback.native_rule = Some(super::native_rules::NativeRule::LionTrial { trial:index as u8 });
        callback.storage = TRACKS[index].iter().enumerate().map(|(i,track)| Storage {
            track:(*track).into(),minimum:0,maximum:if i==0 {i64::from(i32::MAX)-1} else {0},
            set:0,preserve:true,increment:false,at_least:false,
        }).collect();
        callback.passages = vec![Passage {position:Position{x:px,y:py,floor:-8},appearance:2074,replacement:Some(2075),replacement_passable: None,create_on_close:None,if_present:false}];
        callback.item_on_target = Some(ItemOnTarget {item:format!("oteryn:item.tibia.i{}",ITEMS[index]),alternatives:vec![],count:1,replacement:None,target_replacement:None,target_remove:false,reusable:false,selected_instance:false,ingredients:vec![]});
        callback.success_message = Some(MESSAGES[index].into());
        callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/lions_rock/actions_lions_rock.lua:13-79".into()];
        callback
    }).collect()
}

pub(crate) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks.clone();
    for key in FLAGS {
        tracks.insert(
            key.into(),
            QuestTrack {
                key: key.into(),
                quest: QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            },
        );
    }
    QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        catalogue.transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))
}

pub fn apply(questline: i64, flag: i64) -> Result<[i64; 2], QuestRefusal> {
    if flag != 0 {
        return Err(QuestRefusal::StageMismatch);
    }
    let next = questline
        .max(1)
        .checked_add(1)
        .filter(|next| *next <= i64::from(i32::MAX))
        .ok_or(QuestRefusal::OutOfRange)?;
    Ok([next, 1])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lion_trial_preserves_source_counter_and_once_only_flags() {
        assert_eq!(apply(0, 0), Ok([2, 1]));
        assert_eq!(apply(1, 0), Ok([2, 1]));
        assert_eq!(apply(2, 0), Ok([3, 1]));
        assert_eq!(apply(3, 0), Ok([4, 1]));
        assert_eq!(apply(4, 1), Err(QuestRefusal::StageMismatch));
        assert_eq!(apply(i64::from(i32::MAX), 0), Err(QuestRefusal::OutOfRange));
    }
    #[test]
    fn lion_trials_are_closed_native_item_and_pagoda_bindings() -> Result<(), String> {
        for callback in callbacks() {
            super::super::world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
            let mut wrong = callback.clone();
            wrong.item_on_target.as_mut().ok_or("input")?.item = "oteryn:item.tibia.i3595".into();
            assert!(super::super::world_callbacks::validate_callbacks(&[wrong]).is_err());
            let mut wrong = callback;
            wrong.passages[0].position.x += 1;
            assert!(super::super::world_callbacks::validate_callbacks(&[wrong]).is_err());
        }
        Ok(())
    }
}
