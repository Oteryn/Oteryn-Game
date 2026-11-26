use super::{HostOwner, named::NamedControls};
use crate::play::{
    BarAction, ItemUseMode, PassiveAbility, PlayLink, PlayView, SpecialAction, SpellAim,
};
use oteryn_client_runtime::spell::ACTION_BAR_SLOTS;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
};

mod settings;

#[derive(Serialize)]
pub(super) struct LocalState {
    options_page: Option<&'static str>,
    settings: Value,
    arcs: Arcs,
    minimap: Option<Minimap>,
    action_bar: Option<ActionBar>,
}

impl LocalState {
    pub(super) fn read(owner: &HostOwner, named: &NamedControls<'_, '_>) -> Self {
        let preferences = &owner.settings.preferences;
        let mut snapshot = Self {
            options_page: owner.preferences_draft().and_then(|_| {
                if named.full_options() {
                    named.options_page()
                } else {
                    Some("Graphics")
                }
            }),
            settings: settings::read(preferences),
            arcs: Arcs::default(),
            minimap: None,
            action_bar: None,
        };
        if let Some(play) = &owner.play {
            if preferences.health_circle
                && preferences.player_hud_enabled
                && let Some(vitals) = play.link.actor_vitals()
            {
                snapshot.arcs = Arcs {
                    health: Arc::read(
                        preferences.arcs.health,
                        preferences.arcs.opacity,
                        vitals.health,
                        vitals.max_health,
                    ),
                    mana: Arc::read(
                        preferences.arcs.mana,
                        preferences.arcs.opacity,
                        vitals.mana,
                        vitals.max_mana,
                    ),
                };
            }
            snapshot.minimap = Some(Minimap::read(&play.view));
            snapshot.action_bar = Some(ActionBar::read(&play.link, preferences.action_bar.visible));
        }
        snapshot
    }
}

#[derive(Default, Serialize)]
struct Arcs {
    health: Arc,
    mana: Arc,
}

#[derive(Default, Serialize)]
struct Arc {
    shown: bool,
    fill: Option<f32>,
}

impl Arc {
    fn read(enabled: bool, opacity: u8, current: u32, maximum: u32) -> Self {
        if !enabled {
            return Self::default();
        }
        // health_circle::project uses this fraction to draw full and partial segments.
        let fill = if maximum == 0 {
            0.
        } else {
            current.min(maximum) as f32 / maximum as f32
        };
        Self {
            shown: fill > 0. && opacity > 0,
            fill: Some(fill),
        }
    }
}

#[derive(Serialize)]
struct Minimap {
    center: (i32, i32, i16),
    zoom: i32,
    following: bool,
    remembered_tiles: usize,
    marked_tiles: usize,
}

impl Minimap {
    fn read(view: &PlayView) -> Self {
        use oteryn_session::{ActorPosition, world_map::in_window};

        let (x, y, floor, zoom, following) = view.minimap_camera();
        let scene = view.scene();
        let memory = &scene.minimap_memory;
        let (remembered_tiles, marked_tiles) = view.satellite_map(|cells, marks| {
            // As in the renderer, live observations take precedence within the server window.
            let additional_tiles = memory
                .tiles
                .iter()
                .filter(|&&(x, y, floor, _)| {
                    !cells.contains_key(&(floor, x, y))
                        && !scene
                            .minimap_origin
                            .is_some_and(|origin| in_window(origin, ActorPosition { x, y, floor }))
                })
                .count();
            let marked_tiles = marks
                .iter()
                .map(|mark| (mark.x, mark.y, mark.floor))
                .chain(memory.marks.iter().map(|mark| (mark.x, mark.y, mark.floor)))
                .collect::<BTreeSet<_>>()
                .len();
            (cells.len() + additional_tiles, marked_tiles)
        });
        Self {
            center: (x, y, floor),
            zoom,
            following,
            remembered_tiles,
            marked_tiles,
        }
    }
}

#[derive(Serialize)]
struct ActionBar {
    visible_bars: Vec<usize>,
    slots: BTreeMap<usize, ActionIdentity>,
}

impl ActionBar {
    fn read(link: &PlayLink, visible: bool) -> Self {
        link.read_action_bar(|layout, actions, spells, archive| {
            let visible_bars = layout
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.enabled && visible)
                .map(|(index, _)| index)
                .collect();
            let slots = (0..ACTION_BAR_SLOTS)
                .filter_map(|index| {
                    if let Some(action) = actions.0.get(&index) {
                        return Some((index, ActionIdentity::from(action)));
                    }
                    let binding = archive.slots.get(index)?;
                    // The default archive slot is unmanaged and uses the live slot binding.
                    if binding != &Default::default() {
                        return Some((index, ActionIdentity::archived(binding)?));
                    }
                    let slot = NonZeroU32::new(u32::try_from(index + 1).ok()?)?;
                    let cast_index = spells.spell(slot)?;
                    Some((index, ActionIdentity::CastIndex { cast_index }))
                })
                .collect();
            Self {
                visible_bars,
                slots,
            }
        })
    }
}

#[derive(Serialize)]
#[serde(tag = "kind")]
enum ActionIdentity {
    ToggleContainer {
        definition: NonZeroU32,
    },
    Special {
        action: SpecialAction,
    },
    Spell {
        id: String,
        aim: SpellAim,
    },
    ArchiveSpell {
        id: String,
        rune_item: Option<u32>,
    },
    CastIndex {
        cast_index: NonZeroU32,
    },
    Multi {
        actions: Vec<ActionIdentity>,
    },
    Equip {
        definition: NonZeroU32,
        upgrade_tier: u8,
    },
    Text {
        send_automatically: bool,
    },
    Use {
        definition: NonZeroU32,
        sub_type: Option<u32>,
        mode: ItemUseMode,
    },
    Passive {
        ability: PassiveAbility,
    },
}

impl ActionIdentity {
    fn archived(binding: impl Serialize) -> Option<Self> {
        // Archive spell fields are exposed through their preference format.
        let binding = serde_json::to_value(binding).ok()?;
        if binding["kind"] != "Spell" {
            return None;
        }
        let spell = &binding["spell"];
        Some(Self::ArchiveSpell {
            id: spell_id(spell["name"].as_str()?, spell["words"].as_str()?),
            rune_item: spell["rune_item"]
                .as_u64()
                .and_then(|item| u32::try_from(item).ok()),
        })
    }
}

impl From<&BarAction> for ActionIdentity {
    fn from(action: &BarAction) -> Self {
        match action {
            BarAction::ToggleContainer { definition } => Self::ToggleContainer {
                definition: *definition,
            },
            BarAction::Special { action } => Self::Special { action: *action },
            BarAction::Spell { name, words, aim } => Self::Spell {
                id: spell_id(name, words),
                aim: *aim,
            },
            BarAction::Multi { actions } => Self::Multi {
                // The action owner admits at most three leaf actions per slot.
                actions: actions.iter().map(Self::from).collect(),
            },
            BarAction::Equip {
                definition,
                upgrade_tier,
            } => Self::Equip {
                definition: *definition,
                upgrade_tier: *upgrade_tier,
            },
            BarAction::Text {
                send_automatically, ..
            } => Self::Text {
                send_automatically: *send_automatically,
            },
            BarAction::Use {
                definition,
                sub_type,
                mode,
            } => Self::Use {
                definition: *definition,
                sub_type: *sub_type,
                mode: *mode,
            },
            BarAction::Passive { ability } => Self::Passive { ability: *ability },
        }
    }
}

/// Persisted spell names and incantations cannot contain the NUL separator.
fn spell_id(name: &str, words: &str) -> String {
    oteryn_client_assets::text_sha256(format!("{name}\0{words}").as_bytes())
}
