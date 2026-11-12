use super::*;

fn quality_name(quality: u8) -> &'static str {
    ["Lesser", "Regular", "Greater"]
        .get(usize::from(quality))
        .copied()
        .unwrap_or("Unknown")
}

/// Gem Vessel text lines: each slot with its quality and gem, the flat node bonus and the 30%
/// resistance cap notice. `gem_name` renders a gem id.
pub fn vessel_lines(board: &VesselBoard, gem_name: impl Fn([u8; 16]) -> String) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, slot) in board.slots.iter().enumerate() {
        let state = match (slot.open, slot.gem) {
            (false, _) => "locked".to_owned(),
            (true, Some(id)) => gem_name(id),
            (true, None) => "Empty".to_owned(),
        };
        lines.push(format!(
            "Gem Vessel {} ({}): {state}",
            index + 1,
            quality_name(slot.quality)
        ));
    }
    if !board.nodes.is_empty() {
        let slots: Vec<String> = board.nodes.iter().map(u8::to_string).collect();
        lines.push(format!(
            "Gem Vessel nodes sit at Wheel slots {}. Each fully allocated node grants +{} Damage / Healing.",
            slots.join(", "),
            board.node_bonus
        ));
    }
    if board.resistance_capped {
        lines.push(
            "Resistance from socketed gems is capped at 30% against an element; the excess has no effect."
                .to_owned(),
        );
    }
    lines
}
