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
    let Some(capture)=CAPTURE.get_or_init(||serde_json::from_str(include_str!("../../../../../../tools/content-schema/world-authoring/samples/quest-key-door-bindings.json")).ok()) else {return;};
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
    fn source_key_doors_bind_only_the_exact_locked_native_item() {
        for (x, y, floor, action, appearance) in [
            (32190, 32432, -8, 3940, 1631),
            (32277, 32420, -10, 3980, 1631),
            (33211, 31634, -13, 6010, 5097),
            (32462, 32151, -15, 3610, 1628),
            (32462, 32153, -14, 3610, 1631),
            (32592, 32102, -14, 3667, 5106),
        ] {
            let pos = TilePos { x, y, floor };
            let mut attrs = Attrs::default();
            apply(pos, appearance, 0, &mut attrs);
            assert_eq!(attrs.action, Some(action));
            for (position, appearance, house) in [
                (TilePos { x: x + 1, ..pos }, appearance, 0),
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
}
