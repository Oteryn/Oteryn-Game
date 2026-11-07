//! Exact source startup selectors applied without rewriting binary map assets.
use super::{Attrs, TilePos};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Capture {
    schema: u8,
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
        parse(include_str!("../../../../../../tools/content-schema/world-authoring/samples/quest-stock-key-bindings.json"))
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

fn parse(raw: &str) -> Option<Capture> {
    if raw.len() > 1_048_576 {
        return None;
    }
    let capture: Capture = serde_json::from_str(raw).ok()?;
    let unique: std::collections::BTreeSet<_> = capture
        .bindings
        .iter()
        .map(|b| (b.position, b.item, b.attribute.clone()))
        .collect();
    (capture.schema == 1
        && capture.bindings.len() <= 4096
        && unique.len() == capture.bindings.len()
        && capture.bindings.iter().all(|b| {
            b.position[2] <= 15
                && b.item != 0
                && b.value != 0
                && matches!(b.attribute.as_str(), "unique" | "action")
        }))
    .then_some(capture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_stock_keys_bind_only_exact_native_positions_items_and_nonhouses() {
        for (x, y, floor, number) in [
            (32790, 31593, -7, 3003),
            (32807, 31576, -6, 3004),
            (32792, 31590, -6, 3005),
            (32801, 31579, -6, 3006),
            (32813, 31576, -5, 3007),
        ] {
            let pos = TilePos { x, y, floor };
            let mut attrs = Attrs::default();
            apply(pos, 2969, 0, &mut attrs);
            assert_eq!(attrs.action, Some(number));
            for (position, appearance, house) in [
                (TilePos { y: y + 1, ..pos }, 2969, 0),
                (pos, 2970, 0),
                (
                    TilePos {
                        floor: floor - 1,
                        ..pos
                    },
                    2969,
                    0,
                ),
                (pos, 2969, 1),
            ] {
                let mut untouched = Attrs::default();
                apply(position, appearance, house, &mut untouched);
                assert_eq!(untouched.action, None);
            }
            let mut conflicting = Attrs {
                action: Some(42),
                ..Attrs::default()
            };
            apply(pos, 2969, 0, &mut conflicting);
            assert_eq!(conflicting.action, Some(42));
        }
    }

    #[test]
    fn source_binding_capture_is_bounded_and_rejects_ambiguous_or_untyped_attributes() {
        let binding = serde_json::json!({"position":[32790,31593,7],"item":2969,"attribute":"action","value":3003});
        let document = |bindings| serde_json::json!({"schema":1,"bindings":bindings}).to_string();
        assert!(parse(&document(vec![binding.clone()])).is_some());
        assert!(parse(&document(vec![binding.clone(), binding.clone()])).is_none());
        for (field, value) in [
            ("value", serde_json::json!(0)),
            ("item", serde_json::json!(0)),
            ("attribute", serde_json::json!("text")),
            ("position", serde_json::json!([32790, 31593, 16])),
        ] {
            let mut invalid = binding.clone();
            invalid[field] = value;
            assert!(parse(&document(vec![invalid])).is_none());
        }
        assert!(parse(&document(vec![binding; 4097])).is_none());
    }
}
