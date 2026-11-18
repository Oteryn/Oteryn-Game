//! Crystal 201189f8: Putrefactory reservoirs use a persistent counter and per-position clocks.
use super::QUEST;
use crate::quest::QuestEffectKind;
use crate::quest::reward_callbacks::{StateCondition, StateMutation, StateOperation};
use crate::quest::world_callbacks::{Callback, NativeSelector, Position, Storage};

const COUNTER: &str =
    "oteryn:quest-progress/crystalserver/quest/u13_20/rotten_blood/putrefactory_task";
const GATE: &str = "oteryn:quest-callback/rotten-putrefactory-full-counter";
const PREFIX: &str = "oteryn:quest-callback/rotten-putrefactory-reservoir/";
const CLOCK_PREFIX: &str = "oteryn:quest-progress/rotten-blood/putrefactory-reservoir-";
const MAX_POINTS: i64 = 100;
const POINTS_PER_USE: i64 = 20;
// Crystal positions and the exact corresponding objects in the published World bundle.
const RESERVOIRS: [(u16, u16, i8, u8, u16); 6] = [
    (34081, 31704, -14, 2, 43789),
    (34013, 31744, -14, 1, 43790),
    (34044, 31712, -14, 1, 43788),
    (34051, 31706, -15, 2, 43744),
    (34060, 31735, -15, 1, 43787),
    (34095, 31684, -15, 1, 43786),
];

pub(in crate::quest) fn register(callbacks: &mut Vec<Callback>) -> Result<(), String> {
    let gate = callbacks
        .iter_mut()
        .find(|callback| callback.key == GATE)
        .ok_or("missing Putrefactory entry gate")?;
    let [counter] = gate.storage.as_mut_slice() else {
        return Err("invalid Putrefactory entry counter".into());
    };
    if counter.track != COUNTER || counter.minimum != MAX_POINTS {
        return Err("invalid Putrefactory entry counter".into());
    }
    counter.preserve = false;
    gate.requires_world_admission = true;
    gate.selector.native = Some(NativeSelector {
        position: Position {
            x: 34065,
            y: 31718,
            floor: -15,
        },
        ordinal: 1,
        appearance: 10831,
    });
    gate.effects.teleport_effect = Some(12);

    for (index, (x, y, floor, ordinal, appearance)) in RESERVOIRS.into_iter().enumerate() {
        let position = Position { x, y, floor };
        let callback: Callback = serde_json::from_value(serde_json::json!({
            "key": format!("{PREFIX}{}", index + 1),
            "quest": QUEST,
            "trigger": "use",
            "selector": {
                "position": position,
                "action": 47953,
                "native": NativeSelector { position, ordinal, appearance },
            },
            "requires_world_admission": true,
            "reversible": false,
            "passages": [],
            "storage": [{
                "track": COUNTER,
                "minimum": -1,
                "maximum": MAX_POINTS - 1,
                "set": MAX_POINTS
            }],
            "state_operations": [StateOperation {
                track: format!("{CLOCK_PREFIX}{}", index + 1),
                from: StateCondition::ElapsedAtLeast { seconds: 20 },
                effect: StateMutation::SetNow,
            }],
            "effects": {"magic": {"position": position, "effect": 241}},
            "offerings": [],
            "return_on_failure": false,
            "failure_message": "You already exhausted this reservoir.",
            "sources": [
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/rotten_blood_quest/creaturescripts_putrefactory-x.lua:155-224"
            ]
        })).map_err(|error| error.to_string())?;
        super::super::world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
        callbacks.push(callback);
    }
    Ok(())
}

pub(in crate::quest) fn counter_effect(
    callback: &Callback,
    state: &Storage,
    binding: Option<[u8; 32]>,
) -> Option<QuestEffectKind> {
    let binding = binding?;
    if state.track != COUNTER {
        return None;
    }
    if callback.key == GATE {
        Some(QuestEffectKind::AddWithBinding {
            delta: -MAX_POINTS,
            binding,
        })
    } else if callback.key.starts_with(PREFIX) {
        Some(QuestEffectKind::AddCappedWithBinding {
            delta: POINTS_PER_USE,
            maximum: MAX_POINTS,
            binding,
        })
    } else {
        None
    }
}

/// A character may already be pinned to the published, non-spending gate.
/// Preserve the definition from 871c1dec7 without changing stored pins or receipts.
pub(in crate::quest) fn retain_predecessor(catalogue: &mut crate::quest::QuestStateCatalogue) {
    const PUBLISHED_HASH: [u8; 32] = [
        0xd3, 0xb1, 0x44, 0xae, 0x3f, 0x2a, 0x23, 0x84, 0x38, 0x35, 0x84, 0x43, 0x29, 0xa6, 0x8e,
        0x21, 0xce, 0xc9, 0x0e, 0xbc, 0xee, 0x96, 0x5c, 0xb1, 0x31, 0x18, 0x5c, 0x26, 0x5d, 0x5c,
        0x79, 0x3d,
    ];
    if catalogue.definition_hash(QUEST).is_some() {
        catalogue
            .predecessors
            .entry(QUEST.into())
            .or_default()
            .push(crate::quest::NpcDefinitionPredecessor {
                hash: PUBLISHED_HASH,
                defaults: Default::default(),
            });
    }
}
