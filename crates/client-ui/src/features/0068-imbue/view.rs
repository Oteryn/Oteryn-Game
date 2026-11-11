use super::ImbuePanel;
use super::family_name;

#[must_use]
pub fn slot_label(panel: &ImbuePanel, slot: u8) -> String {
    let Some(window) = &panel.window else {
        return String::new();
    };
    if slot >= window.slot_count {
        return String::new();
    }
    window
        .applied
        .iter()
        .find(|applied| applied.slot == slot)
        .map_or_else(
            || lfmt!("Slot {}: empty", slot + 1),
            |applied| {
                lfmt!(
                    "Slot {}: family {}, tier {}, {} seconds left{}",
                    slot + 1,
                    family_name(applied.family),
                    applied.tier,
                    applied.remaining_ms / 1000,
                    time_alert(applied.remaining_ms)
                )
            },
        )
}

#[must_use]
pub fn selected_detail_text(panel: &ImbuePanel) -> String {
    let Some(window) = &panel.window else {
        return tr!("Select an item and refresh its shrine state").into();
    };
    let mut text = lfmt!(
        "Selected {}\nClear cost: {} gold\n",
        slot_label(panel, panel.slot),
        window.clear_gold
    );
    if let Some(offer) = window.offers.get(panel.offer) {
        text.push_str(&lfmt!("Selected offer: family {}, tier {}\nApply cost: {} gold{}\nSuccess chance: 100% (no protection charge)\n", family_name(offer.family), offer.tier, offer.gold, if offer.requires_premium { tr!(" (premium required)") } else { "" }));
        if let Some(quote) = panel
            .details
            .as_ref()
            .filter(|details| Some(details.item) == panel.item)
            .and_then(|details| {
                details
                    .quotes
                    .iter()
                    .find(|quote| quote.family == offer.family && quote.tier == offer.tier)
            })
        {
            text.push_str(&lfmt!(
                "Quoted cost: {} gold; success chance: {}%\n",
                quote.gold,
                quote.success_percent
            ));
            if quote.recipe_available {
                text.push_str(tr!("Required materials (main backpack):\n"));
                for material in &quote.materials {
                    text.push_str(&format!(
                        "  {} × {}\n",
                        material.quantity, material.definition_key
                    ));
                }
            } else {
                text.push_str(tr!("No admitted recipe is available for this offer.\n"));
            }
        } else if panel.details_pending.is_some() {
            text.push_str(tr!("Loading material requirements…\n"));
        } else if let Some(message) = panel.details_feedback {
            text.push_str(message);
            text.push('\n');
        } else {
            text.push_str(tr!("Material details are unavailable in this session.\n"));
        }
    }
    text
}

#[must_use]
pub fn panel_text(panel: &ImbuePanel) -> String {
    let mut text = if let Some(window) = &panel.window {
        lfmt!(
            "{} slots; clear cost: {} gold\n",
            window.slot_count,
            window.clear_gold
        )
    } else {
        tr!("No imbuement state loaded\n").to_owned()
    };
    if let Some(window) = &panel.window {
        for applied in &window.applied {
            text.push_str(&lfmt!(
                "Slot {}: family {}, tier {}, {} ms remaining{}\n",
                applied.slot + 1,
                family_name(applied.family),
                applied.tier,
                applied.remaining_ms,
                time_alert(applied.remaining_ms)
            ));
        }
        for (index, offer) in window.offers.iter().enumerate() {
            text.push_str(&lfmt!(
                "Offer {index}: family {}, tier {}, {} gold{}\n",
                family_name(offer.family),
                offer.tier,
                offer.gold,
                if offer.requires_premium {
                    " (premium)"
                } else {
                    ""
                },
                index = index
            ));
        }
    }
    text.push_str(&selected_detail_text(panel));
    if panel.pending {
        text.push_str(tr!("Request pending\n"));
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

fn time_alert(remaining_ms: u32) -> &'static str {
    match remaining_ms {
        0 => tr!(" — EXPIRED"),
        1..=300_000 => tr!(" — LOW TIME (5 minutes or less)"),
        _ => "",
    }
}

/// The shrine protocol currently confirms one item, so label that scope explicitly.
pub fn tracker_text(panel: &ImbuePanel) -> String {
    let mut text = String::from(if panel.equipment.available {
        "Equipped imbuement tracker\r\n"
    } else {
        tr!("Imbuement time tracker — selected item snapshot\r\n")
    });
    let limit = if panel.tracker_minutes.trim().is_empty() {
        0
    } else {
        let Ok(minutes) = panel.tracker_minutes.trim().parse::<u32>() else {
            return lfmt!("{text}Enter minutes (0 shows all times)", text = text);
        };
        minutes.saturating_mul(60_000)
    };
    if panel.equipment.available {
        return super::equipment_tracker::text(&panel.equipment, limit);
    }
    let Some(window) = &panel.window else {
        text.push_str(
            "Equipment inspection is unavailable; select an item and refresh its server state",
        );
        return text;
    };
    let mut found = false;
    for applied in &window.applied {
        if limit == 0 || applied.remaining_ms <= limit {
            text.push_str(&slot_label(panel, applied.slot));
            text.push_str("\r\n");
            found = true;
        }
    }
    if !found {
        text.push_str(tr!("No confirmed imbuements match this time filter"));
    }
    text
}
