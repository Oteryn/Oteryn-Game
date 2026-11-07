//! ITEM-LOOK-1 item description owner. Target lookup and visibility belong to dispatch.

use oteryn_gameplay_content::content::{
    REFERENCE_ITEM_MAX_DESCRIPTION_BYTES, REFERENCE_ITEM_MAX_NAME_BYTES, ReferenceBaseVocation,
    ReferenceItemField, ReferenceItemSemantics,
};
use std::num::NonZeroU32;

/// Only qualified active-generation content supplies these facts in production.
/// Canary 47dfd51, item.cpp:3322-3347: weight and the definition description are near-only;
/// weapon statistics, requirements and charges are not distance gated.
/// Missing article/plural and instance attributes are never inferred from the name.
pub fn item_text(
    semantics: &ReferenceItemSemantics,
    count: NonZeroU32,
    look_distance: i32,
) -> Option<String> {
    item_text_inner(semantics, count, look_distance, None)
}

/// Base-placement Look only. Borrow and bound untrusted text before copying it.
/// Carried/handle examination continues to use `item_text` without instance text.
pub fn item_text_with_reading(
    semantics: &ReferenceItemSemantics,
    count: NonZeroU32,
    look_distance: i32,
    item_id: Option<u16>,
    instance_text: &str,
    max_text_bytes: usize,
) -> Option<String> {
    let mut end = instance_text.len().min(max_text_bytes);
    while !instance_text.is_char_boundary(end) {
        end -= 1;
    }
    item_text_inner(
        semantics,
        count,
        look_distance,
        Some((item_id, &instance_text[..end])),
    )
}

fn item_text_inner(
    semantics: &ReferenceItemSemantics,
    count: NonZeroU32,
    look_distance: i32,
    reading: Option<(Option<u16>, &str)>,
) -> Option<String> {
    use ReferenceItemField::Known;
    let Known(presentation) = &semantics.presentation else {
        return None;
    };
    let Known(name) = &presentation.name else {
        return None;
    };
    if name.trim().is_empty() || name.len() > REFERENCE_ITEM_MAX_NAME_BYTES {
        return None;
    }
    if matches!(&presentation.description, Known(description)
        if description.len() > REFERENCE_ITEM_MAX_DESCRIPTION_BYTES)
    {
        return None;
    }
    let stack = matches!(&semantics.stack, Known(stack) if stack.stackable == Known(true))
        && count.get() > 1;
    // Presentation currently has no article or plural field. Preserve the admitted name.
    let mut text = if stack {
        format!("You see {count} {name}")
    } else {
        format!("You see {name}")
    };
    let mut attributes = Vec::new();
    if let Known(weapon) = &semantics.weapon {
        if let Known(attack) = weapon.attack
            && attack.0 != 0
        {
            attributes.push(format!("Atk:{}", attack.0));
        }
        if let Known(defense) = weapon.defense {
            let extra = match weapon.extra_defense {
                Known(extra) => extra.0,
                _ => 0,
            };
            if defense.0 != 0 || extra != 0 {
                attributes.push(if extra == 0 {
                    format!("Def:{}", defense.0)
                } else {
                    format!("Def:{} {extra:+}", defense.0)
                });
            }
        }
    }
    if let Known(protection) = &semantics.protection
        && let Known(armor) = protection.armor
        && armor.0 != 0
    {
        attributes.push(format!("Arm:{}", armor.0));
    }
    if let Known(container) = &semantics.container
        && let Known(capacity) = container.capacity
        && capacity != 0
    {
        attributes.push(format!("Vol:{capacity}"));
    }
    if !attributes.is_empty() {
        text.push_str(&format!(" ({})", attributes.join(", ")));
    }
    text.push('.');
    let distance_read =
        matches!(&semantics.readable_writeable, Known(facts) if facts.distance_read == Known(true));
    let raw_text = reading.filter(|(id, _)| distance_read && matches!(id, Some(7369..=7371)));
    if let Some((_, instance)) = reading.filter(|_| distance_read && raw_text.is_none()) {
        text.push('\n');
        if look_distance <= 4 {
            if instance.is_empty() {
                text.push_str("Nothing is written on it.");
            } else {
                text.push_str("You read: ");
                text.push_str(instance);
            }
        } else {
            text.push_str("You are too far away to read it");
            if instance.is_empty() {
                text.push('.');
            }
        }
    }
    // These are definition facts, not remaining instance charges/duration or occupied slots.
    // Content lacks Canary's showCharges/showDuration flags and the observation has no
    // timed-instance snapshot, so do not claim an unused charge or an empty imbuement.
    if let Known(charges) = &semantics.charges
        && let Known(count) = charges.count
    {
        text.push_str(&format!("\nMaximum charges: {count}."));
    }
    if let Known(temporal) = &semantics.temporal
        && let Known(duration) = temporal.duration
    {
        text.push_str(&format!("\nFull duration: {} seconds.", duration.0 / 1000));
    }
    if let Known(equipment) = &semantics.equipment
        && let Known(patterns) = &equipment.patterns
        && let Some(first) = patterns.first()
        && patterns
            .iter()
            .all(|pattern| pattern.level == first.level && pattern.vocations == first.vocations)
    {
        requirements(
            &mut text,
            &first.level,
            &ReferenceItemField::Unknown,
            &first.vocations,
            "wielded properly",
        );
    }
    if let Known(requirement) = &semantics.use_requirements {
        requirements(
            &mut text,
            &requirement.min_level,
            &requirement.min_magic_level,
            &requirement.vocations,
            "used",
        );
    }
    if let Known(imbuement) = &semantics.imbuement
        && let Known(slots) = imbuement.slot_count
        && slots != 0
    {
        text.push_str(&format!("\nImbuement slots: {slots}."));
    }
    if look_distance <= 1 {
        if let Known(physical) = &semantics.physical
            && physical.pickupable == Known(true)
            && let Known(weight) = physical.weight
            && weight != 0
        {
            let weight = u64::from(weight) * if stack { u64::from(count.get()) } else { 1 };
            let subject = if stack { "They weigh" } else { "It weighs" };
            text.push_str(&format!(
                "\n{subject} {}.{:02} oz.",
                weight / 100,
                weight % 100
            ));
        }
        if let Known(description) = &presentation.description
            && !description.is_empty()
        {
            text.push('\n');
            text.push_str(description);
        }
    }
    if let Some((_, instance)) = raw_text.filter(|(_, instance)| !instance.is_empty()) {
        text.push('\n');
        text.push_str(instance);
    }
    Some(text)
}

fn requirements(
    text: &mut String,
    level: &ReferenceItemField<u16>,
    magic_level: &ReferenceItemField<u16>,
    vocations: &ReferenceItemField<Vec<ReferenceBaseVocation>>,
    action: &str,
) {
    use ReferenceItemField::Known;
    let mut details = Vec::new();
    if let Known(level) = level
        && *level != 0
    {
        details.push(format!("level {level} or higher"));
    }
    if let Known(level) = magic_level
        && *level != 0
    {
        details.push(format!("magic level {level} or higher"));
    }
    let names: Vec<_> = match vocations {
        Known(vocations) => vocations
            .iter()
            .map(|vocation| match vocation {
                ReferenceBaseVocation::Druid => "druids",
                ReferenceBaseVocation::Knight => "knights",
                ReferenceBaseVocation::Monk => "monks",
                ReferenceBaseVocation::Paladin => "paladins",
                ReferenceBaseVocation::Sorcerer => "sorcerers",
                ReferenceBaseVocation::None => "players with no vocation",
            })
            .collect(),
        _ => Vec::new(),
    };
    if names.is_empty() && details.is_empty() {
        return;
    }
    let names = if names.is_empty() {
        "players".into()
    } else {
        names.join(" and ")
    };
    text.push_str(&format!("\nIt can only be {action} by {names}"));
    if !details.is_empty() {
        text.push_str(" of ");
        text.push_str(&details.join(" and "));
    }
    text.push('.');
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use oteryn_gameplay_content::content::{ReferenceItemField as F, ReferenceItemPresentation};
    fn semantics(name: F<String>, description: F<String>) -> ReferenceItemSemantics {
        ReferenceItemSemantics {
            presentation: F::Known(ReferenceItemPresentation { name, description }),
            ..Default::default()
        }
    }

    #[test]
    fn formatter_uses_known_text_and_authoritative_count() {
        // ITEM-LOOK-1: count is only shown for known stacks; no separate Count line.
        let facts = semantics(
            F::Known("cheese".into()),
            F::Known("A cave-aged cheese.".into()),
        );
        assert_eq!(
            item_text(&facts, NonZeroU32::new(2).unwrap(), 1),
            Some("You see cheese.\nA cave-aged cheese.".into())
        );
        for unknown in [F::Unknown, F::Conflict, F::NotApplicable] {
            assert_eq!(
                item_text(
                    &semantics(F::Known("cheese".into()), unknown.clone()),
                    NonZeroU32::MIN,
                    1
                ),
                Some("You see cheese.".into())
            );
            assert_eq!(
                item_text(&semantics(unknown, F::Unknown), NonZeroU32::MIN, 1),
                None
            );
        }
        assert_eq!(
            item_text(&ReferenceItemSemantics::default(), NonZeroU32::MIN, 1),
            None
        );
    }

    #[test]
    fn formatter_preserves_existing_content_byte_limits() {
        use oteryn_gameplay_content::content::{
            REFERENCE_ITEM_MAX_DESCRIPTION_BYTES as DESCRIPTION,
            REFERENCE_ITEM_MAX_NAME_BYTES as NAME,
        };
        let valid = semantics(
            F::Known("é".repeat(NAME / 2)),
            F::Known("é".repeat(DESCRIPTION / 2)),
        );
        assert!(item_text(&valid, NonZeroU32::MAX, 1).is_some());
        let bad_name = semantics(F::Known("é".repeat(NAME / 2 + 1)), F::Unknown);
        let bad_description = semantics(
            F::Known("cheese".into()),
            F::Known("é".repeat(DESCRIPTION / 2 + 1)),
        );
        for invalid in [
            bad_name,
            bad_description,
            semantics(F::Known(" ".into()), F::Unknown),
        ] {
            assert_eq!(item_text(&invalid, NonZeroU32::MIN, 1), None);
        }
    }
    #[test]
    fn look_near_far_weight_and_stack_count_preserve_known_presentation() {
        use oteryn_gameplay_content::content::{ReferenceItemPhysical, ReferenceItemStack};
        // A plural supplied as the admitted name is kept; no guessed article or inflection.
        let mut facts = semantics(
            F::Known("berries".into()),
            F::Known("Fresh berries.".into()),
        );
        facts.stack = F::Known(ReferenceItemStack {
            stackable: F::Known(true),
            stack_max: F::Known(100),
        });
        facts.physical = F::Known(ReferenceItemPhysical {
            weight: F::Known(5),
            movable: F::Known(true),
            pickupable: F::Known(true),
        });
        let count = NonZeroU32::new(3).unwrap();
        assert_eq!(item_text(&facts, count, 9).unwrap(), "You see 3 berries.");
        for distance in [-1, 0, 1] {
            assert_eq!(
                item_text(&facts, count, distance).unwrap(),
                "You see 3 berries.\nThey weigh 0.15 oz.\nFresh berries."
            );
        }
        for weight in [1, 9, 10, 99, 100, 12345, u32::MAX] {
            if let F::Known(physical) = &mut facts.physical {
                physical.weight = F::Known(weight);
            }
            assert!(
                item_text(&facts, NonZeroU32::MIN, 1)
                    .unwrap()
                    .contains(&format!(
                        "It weighs {}.{:02} oz.",
                        weight / 100,
                        weight % 100
                    ))
            );
        }
        let max = u64::from(u32::MAX) * u64::from(u32::MAX);
        assert!(
            item_text(&facts, NonZeroU32::MAX, 1)
                .unwrap()
                .contains(&format!("They weigh {}.{:02} oz.", max / 100, max % 100))
        );
        for pickupable in [F::Known(false), F::Unknown, F::Conflict] {
            if let F::Known(physical) = &mut facts.physical {
                physical.pickupable = pickupable;
            }
            assert!(!item_text(&facts, count, 1).unwrap().contains(" oz."));
        }
    }

    #[test]
    fn look_charges_weapon_armor_and_requirements_are_visible_at_distance() {
        use oteryn_gameplay_content::content::{
            ReferenceItemCharges, ReferenceItemImbuement, ReferenceItemProtection,
            ReferenceItemTemporal, ReferenceItemUseRequirements, ReferenceItemWeapon,
            ReferenceMilliseconds, ReferenceSignedPoints, ReferenceUseEnforcementMode,
        };
        let mut facts = semantics(F::Known("sword".into()), F::Unknown);
        facts.weapon = F::Known(ReferenceItemWeapon {
            weapon_type: F::Unknown,
            attack: F::Known(ReferenceSignedPoints(35)),
            defense: F::Known(ReferenceSignedPoints(20)),
            extra_defense: F::Known(ReferenceSignedPoints(2)),
            range: F::Unknown,
            hit_chance: F::Unknown,
            max_hit_chance: F::Unknown,
            ammunition: F::Unknown,
            elemental: F::Unknown,
        });
        facts.protection = F::Known(ReferenceItemProtection {
            armor: F::Known(ReferenceSignedPoints(3)),
            resistances: F::Unknown,
        });
        facts.charges = F::Known(ReferenceItemCharges {
            count: F::Known(10),
        });
        facts.temporal = F::Known(ReferenceItemTemporal {
            consumption_mode: F::Unknown,
            duration: F::Known(ReferenceMilliseconds(120000)),
            stop_duration: F::Unknown,
            decay_target: F::Unknown,
        });
        facts.use_requirements = F::Known(ReferenceItemUseRequirements {
            min_level: F::Known(20),
            min_magic_level: F::Unknown,
            vocations: F::Known(vec![ReferenceBaseVocation::Knight]),
            enforcement_mode: ReferenceUseEnforcementMode::OnUse,
        });
        facts.imbuement = F::Known(ReferenceItemImbuement {
            slot_count: F::Known(2),
            allowed_family_tiers: F::Unknown,
            excluded_families: F::Unknown,
        });
        let expected = "You see sword (Atk:35, Def:20 +2, Arm:3).\nMaximum charges: 10.\nFull duration: 120 seconds.\nIt can only be used by knights of level 20 or higher.\nImbuement slots: 2.";
        for distance in [0, 1, 9, 16] {
            assert_eq!(
                item_text(&facts, NonZeroU32::MIN, distance).unwrap(),
                expected
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod reading_tests {
    use super::*;
    use oteryn_gameplay_content::content::{
        ReferenceItemField as F, ReferenceItemPresentation, ReferenceItemReadableWriteable,
    };

    fn sign() -> ReferenceItemSemantics {
        ReferenceItemSemantics {
            presentation: F::Known(ReferenceItemPresentation {
                name: F::Known("sign".into()),
                description: F::Unknown,
            }),
            readable_writeable: F::Known(ReferenceItemReadableWriteable {
                readable: F::Unknown,
                writeable: F::Unknown,
                distance_read: F::Known(true),
                max_text_length: F::Unknown,
                write_once_target: F::Unknown,
            }),
            ..Default::default()
        }
    }
    fn read(facts: &ReferenceItemSemantics, distance: i32, text: &str) -> String {
        item_text_with_reading(facts, NonZeroU32::MIN, distance, Some(2012), text, 512).unwrap()
    }
    #[test]
    fn reading_near_zero_through_four_including_exactly_four() {
        for distance in 0..=4 {
            assert_eq!(
                read(&sign(), distance, "Go north!\nWelcome"),
                "You see sign.\nYou read: Go north!\nWelcome"
            );
        }
    }
    #[test]
    fn reading_five_and_other_floor_are_too_far() {
        for distance in [5, 15, 19] {
            assert_eq!(
                read(&sign(), distance, "North"),
                "You see sign.\nYou are too far away to read it"
            );
            assert_eq!(
                read(&sign(), distance, ""),
                "You see sign.\nYou are too far away to read it."
            );
        }
    }
    #[test]
    fn reading_empty_text_has_canary_period() {
        for distance in 0..=4 {
            assert_eq!(
                read(&sign(), distance, ""),
                "You see sign.\nNothing is written on it."
            );
        }
    }
    #[test]
    fn reading_non_distance_and_carried_items_keep_existing_description() {
        let mut facts = sign();
        // The handle path has no placement text, even at carried distance -1.
        assert_eq!(
            item_text(&facts, NonZeroU32::MIN, -1).unwrap(),
            "You see sign."
        );
        for gate in [F::Known(false), F::Unknown, F::Conflict, F::NotApplicable] {
            if let F::Known(group) = &mut facts.readable_writeable {
                group.distance_read = gate;
            }
            assert_eq!(read(&facts, 0, "private text"), "You see sign.");
        }
    }
    #[test]
    fn reading_goblet_exception_is_raw_and_not_distance_gated() {
        for id in 7369..=7371 {
            for distance in [0, 4, 5, 19] {
                assert_eq!(
                    item_text_with_reading(
                        &sign(),
                        NonZeroU32::MIN,
                        distance,
                        Some(id),
                        "Winner!",
                        512
                    )
                    .unwrap(),
                    "You see sign.\nWinner!"
                );
                assert_eq!(
                    item_text_with_reading(&sign(), NonZeroU32::MIN, distance, Some(id), "", 512)
                        .unwrap(),
                    "You see sign."
                );
            }
        }
    }
    #[test]
    fn reading_instance_is_bounded_before_copying_on_utf8_boundary() {
        let oversized = "界".repeat(10000);
        let output = read(&sign(), 0, &oversized);
        assert_eq!(
            output,
            format!("You see sign.\nYou read: {}", "界".repeat(170))
        );
        assert!(output.is_char_boundary(output.len()));
    }
}
