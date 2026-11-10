//! Fixed personal Elvenbane exploration from the accepted authored recipe.
//! Exact source region hashes/selectors: questpc2 CONTRACT-NOTES.md.
use super::world_callbacks::{Callback, Position, Selector, Storage, Trigger};
use super::{QuestStateCatalogue, QuestTrack, kill_objectives as rules};

pub const POISON: &str = "oteryn:quest-progress/elvenbane/poison-crossed";
pub const CROSS_POISON: &str = "oteryn:quest-callback/elvenbane/poison";
pub const DRAGON_FLOOR: &str = "oteryn:quest-callback/elvenbane/dragon-floor";

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks.into_values().collect::<Vec<_>>();
    tracks.push(QuestTrack {
        key: POISON.into(),
        quest: rules::QUEST.into(),
        initial: 0,
        min: 0,
        max: 1,
    });
    QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks,
        catalogue.transitions.into_values().collect(),
    )
    .map_err(|error| format!("Elvenbane exploration catalogue: {error:?}"))
}

pub(crate) fn callbacks() -> Vec<Callback> {
    let storage = |track: &str, from, to, preserve| Storage {
        set_if: None,
        track: track.into(),
        minimum: from,
        maximum: from,
        set: to,
        preserve,
        increment: false,
        at_least: false,
        add: None,
        read_only: false,
    };
    [
        (CROSS_POISON, Position {x:32589, y:31644, floor:-6},
            vec![storage(rules::PHASE,1,1,true),storage(POISON,0,1,false)]),
        (DRAGON_FLOOR, Position {x:32588, y:31642, floor:-5},
            vec![storage(rules::PHASE,1,2,false),storage(POISON,1,1,true)]),
    ].into_iter().map(|(key, position, storage)| Callback {
        failure_teleport: None,
        world_change: None,
        remote_teleport: None,
        item_reward: None,
        reward_bundle: false,
        achievements: Vec::new(),
        activation: None,
        requirements: None,
        completes: false,
        actor_kind: Default::default(), opening_only: false, requires_world_admission: false,
        facing: None, world_requirements: Vec::new(),
        key:key.into(), quest:rules::QUEST.into(), trigger:Trigger::StepIn,
        selector:Selector { native: None, position_aliases:Vec::new(),position:Some(position),unique:None,action:None,appearance:None,
ordinal: None,
},
        lever:None,reversible:false,passages:Vec::new(),teleport:None,storage,
        minimum_level:None,experience:None,monk_reward:None,offerings:Vec::new(),
        item_on_target:None,return_on_failure:false,failure_message:None,success_message:None,
        native_rule:None,effects:Default::default(),
        sources:vec!["authored:tools/content-schema/quest-authoring/samples/authored68/recipes.json:2164-2240".into()],
use_range: 1,
disguise: None,
premium_required: false,
present: Vec::new(),
required_travel: Vec::new(),
state_operations: Vec::new(),
}).collect()
}
