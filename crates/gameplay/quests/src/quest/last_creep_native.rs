//! M23 explicitly chosen personal trial. Source identities are exact; geometry and
//! the ItemEx visual bindings are Oteryn choices, not Global behavior claims.
use super::world_callbacks::{Callback, NativeSelector, Position, Selector, Storage, Trigger};
use serde::Deserialize;
use std::sync::OnceLock;

pub const QUEST: &str = "oteryn:quest.authored.last_creep_standing";
const EMBEDDED: &str = include_str!("../../../../../content/quests/last-creep-native.json");

#[derive(Debug, Deserialize)]
pub struct Form {
    pub slug: String,
    pub condition: String,
    pub appearance: u16,
    pub order: i64,
    pub previous: String,
    pub stage: String,
    pub portal: Position,
    pub marker: Position,
    pub callback: String,
}
#[derive(Debug, Deserialize)]
pub struct Document {
    schema: u8,
    basis: String,
    quest: String,
    source_equivalence: bool,
    visual_binding: String,
    pub item: u16,
    pub ground: u16,
    pub portal_appearance: u16,
    pub central: Position,
    pub safe: Position,
    pub return_portal: Position,
    pub exit_portal: Position,
    pub start: Position,
    pub outside: Position,
    pub start_appearance: u16,
    pub forms: Vec<Form>,
    pub sources: Vec<String>,
}

pub fn document() -> Result<&'static Document, String> {
    static DOCUMENT: OnceLock<Result<Document, String>> = OnceLock::new();
    DOCUMENT
        .get_or_init(|| {
            let document: Document = serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
            let ids = [12052, 12047, 12053, 12048, 12056, 12049, 12050, 12051];
            if document.schema != 1
                || document.basis != "CHOSEN_OTERYN_APPROXIMATION"
                || document.quest != QUEST
                || document.source_equivalence
                || document.visual_binding != "CHOSEN_OTERYN_ITEM_EXACT_DUMMY_IDS"
                || document.item != 12251
                || document.ground != 409
                || document.portal_appearance != 1949
                || document.start_appearance != 12047
                || document.forms.len() != ids.len()
                || document.sources.is_empty()
                || document
                    .forms
                    .iter()
                    .zip(ids)
                    .enumerate()
                    .any(|(i, (form, id))| {
                        form.appearance != id
                            || form.order != i as i64 + 1
                            || form.previous != stage(i + 1)
                            || form.stage != stage(i + 2)
                            || form.callback != callback_key(&form.slug)
                            || form.condition
                                != format!("quest.last_creep.{}", form.slug.replace('-', "_"))
                    })
            {
                return Err("invalid chosen Last Creep descriptor".into());
            }
            Ok(document)
        })
        .as_ref()
        .map_err(Clone::clone)
}
fn stage(number: usize) -> String {
    format!("oteryn:quest-progress/authored/last_creep_standing/s{number}")
}
fn callback_key(suffix: &str) -> String {
    format!("oteryn:quest-callback/last-creep-native-{suffix}")
}
fn state(track: String, minimum: i64, maximum: i64, set: i64, preserve: bool) -> Storage {
    Storage {
        set_if: None,
        add: None,
        read_only: false,
        track,
        minimum,
        maximum,
        set,
        preserve,
        increment: false,
        at_least: false,
    }
}
fn callback(
    document: &Document,
    suffix: &str,
    trigger: Trigger,
    position: Position,
    appearance: u16,
    destination: Position,
) -> Callback {
    Callback {
        use_range: 1,
        disguise: None,
        remote_teleport: None,
        state_operations: Vec::new(),
        completes: false,
        premium_required: false,
        present: Vec::new(),
        required_travel: Vec::new(),
        failure_teleport: None,
        item_reward: None,
        reward_bundle: false,
        achievements: Vec::new(),
        activation: None,
        requirements: None,
        track_initials: Default::default(),
        character_requirements: None,
        required_completed_quests: Vec::new(),
        requires_premium: false,
        admission: None,
        key: callback_key(suffix),
        quest: QUEST.into(),
        trigger,
        actor_kind: Default::default(),
        selector: Selector {
        position_aliases: Vec::new(),
        ordinal: None,
            native: Some(NativeSelector {
                position,
                ordinal: 1,
                appearance,
            }),
            position: None,
            unique: None,
            action: None,
            appearance: None,
        },
        lever: None,
        reversible: false,
        opening_only: false,
        requires_world_admission: false,
        passages: Vec::new(),
        teleport: Some(destination),
        facing: None,
        effects: Default::default(),
        storage: Vec::new(),
        state_ops: Vec::new(),
        world_requirements: Vec::new(),
        minimum_level: None,
        world_change: None,
        experience: None,
        monk_reward: None,
        offerings: Vec::new(),
        item_on_target: None,
        return_on_failure: true,
        failure_message: None,
        success_message: None,
        native_rule: None,
        sources: document.sources.clone(),
    }
}

pub fn callbacks() -> Result<Vec<Callback>, String> {
    let document = document()?;
    let mut result = Vec::with_capacity(11);
    let mut start = callback(
        document,
        "start",
        Trigger::Use,
        document.start,
        document.start_appearance,
        document.central,
    );
    start.storage = vec![state(stage(1), 1, 1, 0, true)];
    start.item_on_target = Some(super::world_callbacks::ItemOnTarget {
        on_step: false,
        target_remove: false,
        selected_instance: false,
        ingredients: Vec::new(),
        item: "oteryn:item.tibia.i12251".into(),
        count: 1,
        reusable: true,
        alternatives: Vec::new(),
        replacement: None,
        target_replacement: None,
    });
    start.success_message = Some(
        "Use the central white tile to take your next disguise, then find its matching portal."
            .into(),
    );
    result.push(start);
    for (index, form) in document.forms.iter().enumerate() {
        let mut portal = callback(
            document,
            &form.slug,
            Trigger::StepIn,
            form.portal,
            document.portal_appearance,
            document.safe,
        );
        portal.storage = vec![
            state(form.previous.clone(), 1, 1, 0, true),
            state(form.stage.clone(), 0, 0, 1, false),
        ];
        portal.effects.requires_item_disguise =
            Some(super::world_callbacks::ItemDisguiseRequirement {
                item: format!("oteryn:item.tibia.i{}", form.appearance),
                condition: form.condition.clone(),
            });
        portal.success_message = Some(if index == 7 {
            "You mastered all eight personal forms. The return portal restores your usual appearance.".into()
        } else {
            "Matched. Return to the central chamber for your next form.".into()
        });
        result.push(portal);
    }
    for (suffix, position, destination) in [
        ("return", document.return_portal, document.central),
        ("exit", document.exit_portal, document.outside),
    ] {
        let mut leave = callback(
            document,
            suffix,
            Trigger::StepIn,
            position,
            document.portal_appearance,
            destination,
        );
        leave.effects.restore_disguise = true;
        result.push(leave);
    }
    super::world_callbacks::validate_callbacks(&result)?;
    Ok(result)
}

#[derive(Clone, Copy, Debug)]
pub struct NativeBinding {
    pub position: Position,
    pub ordinal: u8,
    pub appearance: u16,
    /// None is intentional: these are chosen portal visuals, not raw native teleports.
    pub raw_destination: Option<Position>,
}
pub fn native_binding(key: &str) -> Option<NativeBinding> {
    let document = document().ok()?;
    let (position, appearance) = if key == callback_key("start") {
        (document.start, document.start_appearance)
    } else if key == callback_key("return") {
        (document.return_portal, document.portal_appearance)
    } else if key == callback_key("exit") {
        (document.exit_portal, document.portal_appearance)
    } else {
        (
            document
                .forms
                .iter()
                .find(|form| form.callback == key)?
                .portal,
            document.portal_appearance,
        )
    };
    Some(NativeBinding {
        position,
        ordinal: 1,
        appearance,
        raw_destination: None,
    })
}

pub fn disguises() -> Vec<super::item_disguises::Rule> {
    let Ok(document) = document() else {
        return Vec::new();
    };
    document
        .forms
        .iter()
        .map(|form| super::item_disguises::Rule {
            key: form.condition.clone(),
            sources: document.sources.clone(),
            item: "oteryn:item.tibia.i12251".into(),
            appearance: form.appearance,
            track: form.stage.clone(),
            required: 0,
            requires: vec![super::reward_callbacks::TrackRequirement {
                track: form.previous.clone(),
                minimum: 1,
                maximum: 1,
            }],
            activation: Some(document.central),
            restore: None,
            use_text: format!(
                "You take the {} disguise. Find the matching portal.",
                form.slug.replace('-', " ")
            ),
            restore_text: String::new(),
            use_effect: 3,
            speech_type: 36,
        })
        .collect()
}

/// All form journal state uses canonical s2..s9 directly. No parallel phase counter.
/// Install existing-owner tracks before world_callbacks::extend.
/// s10..s12 and reward/completion holds remain untouched.
pub fn extend(catalogue: super::QuestStateCatalogue) -> Result<super::QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks.clone();
    for (key, maximum) in (1..=9).map(|number| (stage(number), 1)) {
        if let Some(existing) = tracks.get(&key) {
            if existing.quest != QUEST
                || existing.initial != 0
                || existing.min != 0
                || existing.max != maximum
            {
                return Err(format!("Last Creep track ownership differs: {key}"));
            }
        } else {
            tracks.insert(
                key.clone(),
                super::QuestTrack {
                    key,
                    quest: QUEST.into(),
                    initial: 0,
                    min: 0,
                    max: maximum,
                },
            );
        }
    }
    super::QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        catalogue.transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m23_last_creep_disguise_gates_require_player_steps_and_teleports() -> Result<(), String> {
        let callbacks = callbacks()?;
        let portal = callbacks
            .iter()
            .find(|callback| callback.effects.requires_item_disguise.is_some())
            .ok_or("matching portal")?;
        let mut use_gate = portal.clone();
        use_gate.trigger = Trigger::Use;
        let mut no_landing = portal.clone();
        no_landing.teleport = None;
        let mut monster_gate = portal.clone();
        monster_gate.actor_kind = super::super::world_callbacks::ActorKind::Monster;
        let mut empty_condition = portal.clone();
        empty_condition
            .effects
            .requires_item_disguise
            .as_mut()
            .ok_or("condition gate")?
            .condition
            .clear();
        for invalid in [use_gate, no_landing, monster_gate, empty_condition] {
            assert!(super::super::world_callbacks::validate_callbacks(&[invalid]).is_err());
        }
        let exit = callbacks
            .iter()
            .find(|callback| callback.effects.restore_disguise)
            .ok_or("restoration exit")?;
        let mut use_exit = exit.clone();
        use_exit.trigger = Trigger::Use;
        let mut no_exit_landing = exit.clone();
        no_exit_landing.teleport = None;
        for invalid in [use_exit, no_exit_landing] {
            assert!(super::super::world_callbacks::validate_callbacks(&[invalid]).is_err());
        }
        Ok(())
    }

    #[test]
    fn m23_last_creep_uses_fixed_forms_and_bare_exact_step_bindings() -> Result<(), String> {
        let descriptor = document()?;
        let callbacks = callbacks()?;
        assert_eq!(callbacks.len(), 11);
        assert_eq!(disguises().len(), 8);
        for callback in &callbacks {
            let binding = native_binding(&callback.key).ok_or("missing native binding")?;
            assert_eq!(binding.raw_destination, None);
            let selector = callback.selector.native.as_ref().ok_or("native selector")?;
            assert_eq!(
                (selector.position, selector.ordinal, selector.appearance),
                (binding.position, 1, binding.appearance)
            );
            assert!(
                callback.selector.position.is_none()
                    && callback.selector.unique.is_none()
                    && callback.selector.action.is_none()
                    && callback.selector.appearance.is_none()
            );
        }
        for form in &descriptor.forms {
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == form.callback)
                .ok_or("form")?;
            let required = callback
                .effects
                .requires_item_disguise
                .as_ref()
                .ok_or("condition gate")?;
            assert_eq!(
                required.item,
                format!("oteryn:item.tibia.i{}", form.appearance)
            );
            assert_eq!(required.condition, form.condition);
            assert_eq!(callback.teleport, Some(descriptor.safe));
        }
        let start = callbacks
            .iter()
            .find(|callback| callback.key == callback_key("start"))
            .ok_or("start")?;
        assert!(
            start.storage.iter().any(|state| state.track == stage(1)
                && state.preserve
                && (state.minimum, state.maximum) == (1, 1)),
            "M23 must not manufacture Casper TALK progress"
        );
        assert!(
            callbacks
                .iter()
                .flat_map(|callback| &callback.storage)
                .all(|state| ![stage(10), stage(11), stage(12)].contains(&state.track))
        );
        Ok(())
    }
}
