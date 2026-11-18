//! Crystal 201189f creaturescripts_kill.lua:10-39 retains the highest defeated rank.
use super::{Comparison, Document, Effect, Rule};

pub(super) const QUEST: &str = "oteryn:quest.the_secret_library_quest";
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
        tracks: Vec::new(),
    }
}

/// Append source mechanics after the usual catalogue assembly, retaining the exact
/// previous definition for characters with existing Secret Library progress.
pub(in crate::quest) fn extend(
    base: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    use crate::quest::{NpcDefinitionPredecessor, QuestTrack};
    use sha2::{Digest, Sha256};
    let mut tracks = base.tracks.clone();
    if let Some(track) = tracks.get(TRACK) {
        if track.quest != QUEST || track.max < 5 {
            return Err("Falcon rank track conflict".into());
        }
    } else {
        tracks.insert(
            TRACK.into(),
            QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: 6,
            },
        );
    }
    let mut transitions = base.transitions.clone();
    for rule in super::rules()
        .map_err(str::to_owned)?
        .iter()
        .filter(|rule| rule.quest == QUEST)
    {
        if transitions
            .insert(rule.transition.clone(), rule.transition()?)
            .is_some()
        {
            return Err("Falcon rank transition conflict".into());
        }
    }
    let mut extended = super::QuestStateCatalogue::new(
        base.content_revision.clone(),
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let rank_binding = extended
        .definition_hash(QUEST)
        .ok_or("Falcon rank definition")?;
    let prior = base.definition_hash(QUEST);
    extended.hashes.extend(base.hashes);
    extended.completion_sets = base.completion_sets;
    extended.npc_legacy = base.npc_legacy;
    extended.predecessors = base.predecessors;
    extended.authored_bindings = base.authored_bindings;
    extended.native_recipe_evidence = base.native_recipe_evidence;
    if let Some(hash) = prior {
        extended
            .predecessors
            .entry(QUEST.into())
            .or_default()
            .push(NpcDefinitionPredecessor {
                hash,
                defaults: Default::default(),
            });
        extended.hashes.insert(
            QUEST.into(),
            Sha256::new()
                .chain_update(b"falcon-rank-v1")
                .chain_update(hash)
                .chain_update(rank_binding)
                .finalize()
                .into(),
        );
    }
    Ok(extended)
}
