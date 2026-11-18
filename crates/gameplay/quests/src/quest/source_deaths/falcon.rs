//! Crystal 201189f creaturescripts_kill.lua:10-39 retains the highest defeated rank.
use super::{Comparison, Document, Effect, Rule, Track};

const QUEST: &str = "oteryn:quest.the_secret_library_quest";
const TRACK: &str =
    "oteryn:quest-progress/quest/u11_80/the_secret_library/falcon_bastion/killing_bosses";
const BOSSES: [(&str, i64); 5] = [
    ("dazed_leaf_golem", 5),
    ("grand_canon_dominus", 4),
    ("grand_chaplain_gaunder", 3),
    ("grand_commander_soeren", 1),
    ("preceptor_lazare", 2),
];

pub(super) fn document() -> Document {
    Document {
        schema: 1,
        runtime_enabled: true,
        quest: QUEST.into(),
        rules: BOSSES
            .iter()
            .map(|(boss, rank)| Rule {
                creature: format!("oteryn:creature.{boss}"),
                transition: format!("oteryn:quest-transition/falcon-bastion/{boss}"),
                effects: vec![Effect {
                    track: TRACK.into(),
                    default: -1,
                    from: Comparison {
                        kind: "lt".into(),
                        value: *rank,
                    },
                    set: Some(*rank),
                    add: None,
                }],
                room: None,
                top_damage_only: false,
                wild_only: true,
                define_transition: true,
                // Credit never resets: each queued death compares against the locked rank,
                // including writes by another death captured from the same quest snapshot.
                epoch_tracks: Some(Vec::new()),
                quest: String::new(),
            })
            .collect(),
        tracks: vec![Track {
            key: TRACK.into(),
            initial: -1,
            min: -1,
            max: 6,
        }],
    }
}
