//! Exact source startup selectors applied without rewriting binary map assets.
use super::{Attrs, TilePos};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Capture {
    bindings: Vec<Binding>,
}
#[derive(Deserialize)]
struct Binding {
    position: [u16; 3],
    item: u16,
    attribute: String,
    value: u16,
}

pub(super) fn apply(position: TilePos, appearance: u16, house: u32, attrs: &mut Attrs) {
    static CAPTURE: OnceLock<Option<Capture>> = OnceLock::new();
    let Some(capture) = CAPTURE.get_or_init(|| {
        let mut capture: Capture = serde_json::from_str(include_str!(
            "../../../../../../tools/content-schema/world-authoring/samples/quest-access-bindings.json"
        )).ok()?;
        let key_doors: Capture = serde_json::from_str(include_str!(
            "../../../../../../tools/content-schema/world-authoring/samples/quest-key-door-bindings.json"
        )).ok()?;
        let classic_quests: Capture = serde_json::from_str(include_str!(
            "../../../../../../tools/content-schema/world-authoring/samples/classic-quest-startup.json"
        )).ok()?;
        capture.bindings.extend(classic_quests.bindings);
        capture.bindings.extend(key_doors.bindings);
        Some(capture)
    }) else {
        return;
    };
    if house != 0 {
        return;
    }
    for binding in &capture.bindings {
        if binding.item != appearance
            || binding.position[0] != position.x
            || binding.position[1] != position.y
            || i32::from(binding.position[2]) != -i32::from(position.floor)
        {
            continue;
        }
        let field = match binding.attribute.as_str() {
            "unique" => &mut attrs.unique,
            "action" => &mut attrs.action,
            _ => continue,
        };
        // Conflicting native attributes remain authoritative and cannot match this binding.
        if field.is_none() {
            *field = Some(binding.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_routes_bind_only_the_exact_native_item() {
        for (x, y, floor, appearance, action) in [
            (32190, 32432, -8, 1631, 3940),
            (32277, 32420, -10, 1631, 3980),
            (32171, 32439, -7, 1930, 15005),
            (32172, 32439, -7, 1931, 15005),
            (32212, 32435, -10, 1648, 1050),
        ] {
            let pos = TilePos { x, y, floor };
            let mut attrs = Attrs::default();
            apply(pos, appearance, 0, &mut attrs);
            assert_eq!(attrs.action, Some(action));
            for (position, appearance, house) in [
                (TilePos { y: y + 1, ..pos }, appearance, 0),
                (pos, appearance + 1, 0),
                (pos, appearance, 1),
            ] {
                let mut untouched = Attrs::default();
                apply(position, appearance, house, &mut untouched);
                assert_eq!(untouched.action, None);
            }
            let mut conflicting = Attrs {
                action: Some(42),
                ..Attrs::default()
            };
            apply(pos, appearance, 0, &mut conflicting);
            assert_eq!(conflicting.action, Some(42));
        }
    }

    #[test]
    fn classic_quest_startup_selectors_bind_only_the_donor_objects() {
        for (x, y, floor, item, action, unique) in [
            (31973, 32905, -11, 19387, Some(25018), None),
            (31973, 32905, -10, 19387, Some(25018), None),
            (32009, 32928, -9, 19387, Some(25018), None),
            (32009, 32928, -10, 19387, Some(25018), None),
            (32017, 32917, -10, 104, Some(25018), None),
            (32017, 32918, -10, 104, Some(25018), None),
            (31974, 32907, -8, 4285, Some(40001), None),
            (32031, 32914, -8, 4285, Some(40001), None),
            (32022, 32918, -10, 21572, Some(40002), None),
            (33109, 32530, -3, 1928, Some(12105), None),
            (33094, 32524, -1, 3263, None, Some(2285)),
        ] {
            let position = TilePos { x, y, floor };
            let mut attrs = Attrs::default();
            apply(position, item, 0, &mut attrs);
            assert_eq!(
                (attrs.action, attrs.unique),
                (action, unique),
                "{position:?}"
            );
            let mut unrelated = Attrs::default();
            apply(position, item + 1, 0, &mut unrelated);
            assert_eq!((unrelated.action, unrelated.unique), (None, None));
        }
    }

    #[test]
    fn pinned_selector_requires_exact_position_item_and_no_house() {
        let pos = TilePos {
            x: 32227,
            y: 32278,
            floor: -8,
        };
        let mut attrs = Attrs::default();
        apply(pos, 2772, 0, &mut attrs);
        assert_eq!(attrs.unique, Some(30004));
        apply(pos, 2772, 0, &mut attrs);
        assert_eq!(attrs.unique, Some(30004));
        for (p, item, house) in [
            (TilePos { x: 32228, ..pos }, 2772, 0),
            (pos, 2773, 0),
            (pos, 2772, 1),
        ] {
            let mut untouched = Attrs::default();
            apply(p, item, house, &mut untouched);
            assert_eq!(untouched.unique, None);
        }
    }
    #[test]
    fn conflicting_native_selector_is_preserved() {
        let mut attrs = Attrs {
            unique: Some(42),
            ..Attrs::default()
        };
        apply(
            TilePos {
                x: 32227,
                y: 32278,
                floor: -8,
            },
            2772,
            0,
            &mut attrs,
        );
        assert_eq!(attrs.unique, Some(42));
    }
}
