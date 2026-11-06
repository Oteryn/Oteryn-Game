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
        serde_json::from_str(include_str!(
            "../../../../../../tools/content-schema/world-authoring/samples/quest-access-bindings.json"
        ))
        .ok()
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
