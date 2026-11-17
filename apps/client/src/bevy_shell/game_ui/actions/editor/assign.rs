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
    if !actions.is_empty()
        && link.assign_bar_action(
            slot,
            BarAction::Multi {
                actions: actions.clone(),
            },
        )
    {
        return tr!("Multiaction saved").into();
    }
    let message = lfmt!(
        "Assign at least one live spell or carried object.\n{}",
        multi_message(&actions)
    );
    *draft = Some(actions);
    message
}

/// The object action the item fields describe, or `None` when they hold no valid object.
fn object_action(action: Edit, values: &[String]) -> Option<BarAction> {
    let definition = values.get(1)?.trim().parse::<std::num::NonZeroU32>().ok()?;
    let sub_type = match values
        .get(4)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        Some(value) => Some(value.parse::<u32>().ok()?),
        None => None,
    };
    Some(if matches!(action, Edit::Equip) {
        BarAction::Equip {
            definition,
            upgrade_tier: 0,
        }
    } else {
        BarAction::Use {
            definition,
            sub_type,
            mode: match action {
                Edit::SelfActor => ItemUseMode::SelfActor,
                Edit::Target => ItemUseMode::Target,
                Edit::Cursor => ItemUseMode::Cursor,
                Edit::Use => ItemUseMode::Use,
                _ => ItemUseMode::CursorPosition,
            },
        }
    })
}

/// Applies one editor button; a multiaction draft collects objects instead of assigning them.
pub(super) fn apply(
    link: &crate::play::PlayLink,
    slot: usize,
    action: Edit,
    values: &[String],
    draft: &mut Option<Vec<BarAction>>,
) -> String {
    if slot >= oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
        return REFUSED.into();
    }
    let saved = match action {
        Edit::AimTarget | Edit::AimFacing | Edit::AimCursor => link.bar_gesture(BarGesture::Aim {
            slot,
            aim: match action {
                Edit::AimTarget => crate::play::SpellAim::Target,
                Edit::AimFacing => crate::play::SpellAim::Facing,
                _ => crate::play::SpellAim::Cursor,
            },
        }),
        Edit::Key => link.bind_action_key(slot, &values[3]),
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
            let Some(object) = object_action(action, values) else {
                return REFUSED.into();
            };
            if let Some(actions) = draft {
                return add_to_multi(actions, object);
            }
            link.assign_bar_action(slot, object)
        }
        Edit::Close | Edit::PreviousSpells | Edit::NextSpells | Edit::Multi => false,
    };
    String::from(if saved { SAVED } else { REFUSED })
}
