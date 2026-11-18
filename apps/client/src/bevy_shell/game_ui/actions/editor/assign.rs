//! Applying an edited action-bar assignment and the I-III multi-action draft.
use super::*;

pub(super) const REFUSED: &str =
    "Choose a valid live spell, carried item, text or unused keyboard chord.";
const SAVED: &str = "Saved action assignment";

/// The draft's slots I-III and how the window uses them.
fn multi_message(actions: &[BarAction]) -> String {
    let slots = ["I", "II", "III"]
        .iter()
        .enumerate()
        .map(|(index, numeral)| {
            format!(
                "{numeral}: {}",
                actions
                    .get(index)
                    .map_or_else(|| "Empty".into(), action_label)
            )
        })
        .collect::<Vec<_>>()
        .join("   ");
    format!(
        "Multiaction {slots}\nChoose up to three spells or objects, then press Multiaction to save. \
         The first action off cooldown will execute."
    )
}

fn action_label(action: &BarAction) -> String {
    match action {
        BarAction::Spell { name, .. } => name.clone(),
        BarAction::Use { definition, .. } => {
            lfmt!("Use object {definition}", definition = definition)
        }
        BarAction::Equip { definition, .. } => {
            lfmt!("Equip object {definition}", definition = definition)
        }
        BarAction::Text { text, .. } => text.clone(),
        BarAction::Passive { .. } => tr!("Passive ability").into(),
        BarAction::Multi { actions } => lfmt!("Multiaction ({} actions)", actions.len()),
    }
}

/// Adds one spell or object to the draft, at most three.
pub(super) fn add_to_multi(draft: &mut Vec<BarAction>, action: BarAction) -> String {
    if draft.len() >= 3 {
        return tr!("A multiaction holds at most three actions.").into();
    }
    draft.push(action);
    multi_message(draft)
}

/// Starts a draft from the slot's saved multiaction, or saves the draft into the slot.
pub(super) fn toggle_multi(
    link: &crate::play::PlayLink,
    slot: usize,
    saved: Option<&BarAction>,
    draft: &mut Option<Vec<BarAction>>,
) -> String {
    let Some(actions) = draft.take() else {
        let actions = match saved {
            Some(BarAction::Multi { actions }) => actions.clone(),
            _ => Vec::new(),
        };
        let message = multi_message(&actions);
        *draft = Some(actions);
        return message;
    };
    let tiered = link.hotkey_equip_panel().tiered;
    let supported = actions
        .iter()
        .all(|action| action.equip_tier_supported(tiered));
    if supported
        && !actions.is_empty()
        && link.assign_bar_action(
            slot,
            BarAction::Multi {
                actions: actions.clone(),
            },
        )
    {
        return tr!("Multiaction saved").into();
    }
    let message = if supported {
        lfmt!(
            "Assign at least one live spell or carried object.\n{}",
            multi_message(&actions)
        )
    } else {
        format!(
            "This world does not support exact upgrade tiers.\n{}",
            multi_message(&actions)
        )
    };
    *draft = Some(actions);
    message
}

/// The object action the item fields describe, or `None` when they hold no valid object.
fn object_action(action: Edit, values: &[String]) -> Option<BarAction> {
    let definition = values.get(1)?.trim().parse::<std::num::NonZeroU32>().ok()?;
    if matches!(action, Edit::Equip) {
        return Some(BarAction::Equip {
            definition,
            upgrade_tier: 0,
        });
    }
    let sub_type = match values
        .get(4)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        Some(value) => Some(value.parse::<u32>().ok()?),
        None => None,
    };
    Some(BarAction::Use {
        definition,
        sub_type,
        mode: match action {
            Edit::SelfActor => ItemUseMode::SelfActor,
            Edit::Target => ItemUseMode::Target,
            Edit::Cursor => ItemUseMode::Cursor,
            Edit::Use => ItemUseMode::Use,
            _ => ItemUseMode::CursorPosition,
        },
    })
}

/// Applies one editor button; a multiaction draft collects objects instead of assigning them.
pub(super) fn apply(
    owner: &mut HostOwner,
    slot: usize,
    action: Edit,
    values: &[String],
    draft: &mut Option<Vec<BarAction>>,
) -> String {
    if slot >= oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
        return REFUSED.into();
    }
    let Some(play) = &owner.play else {
        return REFUSED.into();
    };
    let link = &play.link;
    let saved = match action {
        Edit::AimTarget | Edit::AimFacing | Edit::AimCursor => link.bar_gesture(BarGesture::Aim {
            slot,
            aim: match action {
                Edit::AimTarget => crate::play::SpellAim::Target,
                Edit::AimFacing => crate::play::SpellAim::Facing,
                _ => crate::play::SpellAim::Cursor,
            },
        }),
        Edit::Key => {
            owner.settings.sync_bar_actions(link);
            let mut prefs = owner.settings.preferences.clone();
            let Some(binding) = slot_binding(slot) else {
                return REFUSED.into();
            };
            let chord = values[3].trim().to_ascii_uppercase();
            if matches!(
                chord.as_str(),
                "ESCAPE" | "ENTER" | "KEYPADENTER" | "NUMPADENTER"
            ) {
                return REFUSED.into();
            }
            let chord = if chord.is_empty() {
                binding.default_chord
            } else {
                &chord
            };
            let live_context = link.preference_chat_open();
            if !binding.is_extra_action_bar_slot() {
                if prefs.action_keys.0.get(&slot).is_some_and(|saved| {
                    saved.trim().eq_ignore_ascii_case(chord)
                        || slot_chord(&prefs, &binding, slot, live_context)
                            .trim()
                            .eq_ignore_ascii_case(chord)
                }) {
                    return SAVED.into();
                }
                prefs.action_keys.0.remove(&slot);
            }
            let options_context = prefs.hotkey_contexts.chat_on;
            let switch_context = !binding.is_extra_action_bar_slot()
                && !prefs.manual_hotkey_profile
                && !prefs.chat_context_hotkeys
                && options_context != live_context;
            if (switch_context && prefs.select_hotkey_context(live_context).is_err())
                || prefs.rebind(binding.id, chord).is_err()
                || (switch_context && prefs.select_hotkey_context(options_context).is_err())
            {
                return REFUSED.into();
            }
            owner.settings.apply(prefs);
            owner.settings.sync_bar_actions(link);
            true
        }
        Edit::Clear => {
            *draft = None;
            link.clear_bar_action(slot);
            link.assign_spell_slot(slot, None)
        }
        Edit::Text | Edit::InsertText => link.assign_bar_action(
            slot,
            BarAction::Text {
                text: values[2].clone(),
                send_automatically: matches!(action, Edit::Text),
            },
        ),
        Edit::Instinct | Edit::Harmony | Edit::Serene => link.assign_bar_action(
            slot,
            BarAction::Passive {
                ability: match action {
                    Edit::Instinct => PassiveAbility::BattleInstinct,
                    Edit::Harmony => PassiveAbility::Harmony,
                    _ => PassiveAbility::Serene,
                },
            },
        ),
        Edit::Equip
        | Edit::Use
        | Edit::CursorPosition
        | Edit::SelfActor
        | Edit::Target
        | Edit::Cursor => {
            let Some(mut object) = object_action(action, values) else {
                return REFUSED.into();
            };
            if let BarAction::Equip { upgrade_tier, .. } = &mut object {
                let entered = values[5].trim();
                if link.hotkey_equip_panel().tiered {
                    let Some(exact) = entered.parse::<u8>().ok().filter(|&tier| {
                        tier <= oteryn_client_runtime::cui3::hotkey_equip::MAX_UPGRADE_TIER
                    }) else {
                        return "Enter the exact upgrade tier from 0 to 10.".into();
                    };
                    *upgrade_tier = exact;
                } else if !entered.is_empty() {
                    return "This world does not support exact upgrade tiers. Leave the tier blank.".into();
                }
            }
            if let Some(actions) = draft {
                return add_to_multi(actions, object);
            }
            link.assign_bar_action(slot, object)
        }
        Edit::Close | Edit::PreviousSpells | Edit::NextSpells | Edit::Multi => false,
    };
    String::from(if saved { SAVED } else { REFUSED })
}
