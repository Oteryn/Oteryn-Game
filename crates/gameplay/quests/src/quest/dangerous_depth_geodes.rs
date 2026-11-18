//! Crystal 201189f8, actions_using_crystals.lua:416–530.
use super::world_callbacks::{Callback, ItemOnTarget, NativeSelector, Position, Selector};

pub const FRAGMENT: &str = "oteryn:item.tibia.i27509";
const PREFIX: &str = "oteryn:quest-callback/dangerous-depth-geode-";

#[derive(Clone, Copy)]
pub struct Geode {
    suffix: &'static str,
    source: Position,
    ordinal: u8,
    pub entrance: Position,
    pub entrance_ordinal: u8,
    pub landing: Position,
}

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -15 }
}

const GEODES: [Geode; 3] = [
    Geode {
        suffix: "iv",
        source: at(33459, 32267),
        ordinal: 2,
        entrance: at(33460, 32267),
        entrance_ordinal: 1,
        landing: at(33650, 32310),
    },
    Geode {
        suffix: "v",
        source: at(33323, 32109),
        ordinal: 2,
        entrance: at(33324, 32109),
        entrance_ordinal: 2,
        landing: at(33681, 32338),
    },
    Geode {
        suffix: "vi",
        source: at(33274, 32316),
        ordinal: 1,
        entrance: at(33275, 32316),
        entrance_ordinal: 1,
        landing: at(33717, 32302),
    },
];

pub fn binding(key: &str) -> Option<Geode> {
    let suffix = key.strip_prefix(PREFIX)?;
    GEODES.into_iter().find(|geode| geode.suffix == suffix)
}

pub(crate) fn callbacks() -> impl Iterator<Item = Callback> {
    GEODES.into_iter().map(Geode::callback)
}

pub fn shared_input(callback: &Callback) -> bool {
    binding(&callback.key).is_some_and(|geode| geode.callback() == *callback)
}

pub fn shared_transition(transition: &super::QuestTransition) -> bool {
    binding(&transition.key).is_some_and(|geode| {
        super::world_callbacks::primary_transition(&geode.callback())
            .is_ok_and(|expected| expected == *transition)
    })
}

impl Geode {
    fn callback(self) -> Callback {
        let mut callback = super::source_world_objects::uneasy_crystal();
        callback.key = format!("{PREFIX}{}", self.suffix);
        callback.quest = "oteryn:quest.dangerous_depths_quest".into();
        callback.selector = Selector {
            native: Some(NativeSelector {
                position: self.source,
                ordinal: self.ordinal,
                appearance: 27510,
            }),
            position: None,
            position_aliases: Vec::new(),
            appearance: None,
            ordinal: None,
            action: None,
            unique: None,
        };
        callback.storage.clear();
        callback.effects = Default::default();
        callback.requires_world_admission = true;
        callback.item_on_target = Some(ItemOnTarget {
            on_step: false,
            item: FRAGMENT.into(),
            alternatives: Vec::new(),
            count: 1,
            replacement: None,
            target_replacement: None,
            reusable: false,
            selected_instance: true,
            target_remove: false,
            ingredients: Vec::new(),
        });
        callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dangerous_depth/actions_using_crystals.lua:416-530".into()];
        callback
    }
}
