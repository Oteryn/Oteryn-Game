//! Costs reflect the accepted shrine contract; never infer a wallet or sell protection.
use super::*;

pub(super) fn summary(panel: &ImbuePanel, selected: Option<usize>) -> String {
    let mut text = String::from(
        "Balance: not supplied by this session; affordability is checked by the server.\n\
         Protection: unavailable in this session; no protection charge.\n",
    );
    let Some(offer) = selected
        .filter(|index| *index == panel.offer)
        .and_then(|index| panel.window.as_ref()?.offers.get(index))
    else {
        text.push_str("Select an imbuement to review its apply cost and success chance.");
        return text;
    };
    text.push_str(&format!(
        "Apply total: {} gold + required materials{}\n",
        offer.gold,
        if offer.requires_premium {
            " (premium required)"
        } else {
            ""
        },
    ));
    let quote = panel
        .details
        .as_ref()
        .filter(|details| Some(details.item) == panel.item)
        .and_then(|details| {
            details
                .quotes
                .iter()
                .find(|quote| quote.family == offer.family && quote.tier == offer.tier)
        });
    let chance = oteryn_client_ui::imbuement_success_chance::success_percent(panel, panel.offer)
        .or_else(|| quote.map(|quote| quote.success_percent));
    if let Some(chance) = chance {
        text.push_str(&format!(
            "Success: {chance}% — failure: {}%\n",
            100u8.saturating_sub(chance)
        ));
    } else {
        // IMBUE_FORGE0 R2: legacy sessions always succeed without protection.
        text.push_str("Success: 100% (standard session rules)\n");
    }
    text.push_str(if quote.is_some() {
        "Material quantities below are requirements, not your available stock."
    } else if panel.details_pending.is_some() {
        "Loading material requirements…"
    } else {
        "Material requirements are unavailable in this session."
    });
    text
}
