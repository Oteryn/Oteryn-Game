use crate::imbue::{ImbuePanel, ImbueResponse, imbuement};

pub fn apply_result(panel: &mut ImbuePanel, response: &ImbueResponse) {
    panel.pending = false;
    let Some(reply) = &response.reply else {
        panel.feedback = Some(tr!("The server refused the imbuement request"));
        return;
    };
    panel.feedback = match reply.result {
        imbuement::ImbuementResult::Ok => None,
        imbuement::ImbuementResult::RequirementNotMet => Some(tr!("Requirements are not met")),
        imbuement::ImbuementResult::MaterialsMissing => Some(tr!("Required materials are missing")),
        imbuement::ImbuementResult::InsufficientFunds => Some(tr!("Insufficient funds")),
        imbuement::ImbuementResult::SlotOccupied => Some(tr!("That slot is occupied")),
        imbuement::ImbuementResult::NotAllowed => Some(tr!("This imbuement is not allowed")),
        imbuement::ImbuementResult::RevisionChanged => Some(tr!("The item changed; refresh it")),
    };
    if let Some(window) = &reply.window {
        panel.window = Some(window.clone());
        panel.details = None;
        panel.details_pending = None;
        panel.details_attempted = false;
        panel.details_feedback = None;
        panel.slot = panel.slot.min(window.slot_count.saturating_sub(1));
        panel.offer = panel.offer.min(window.offers.len().saturating_sub(1));
    } else if matches!(
        reply.result,
        imbuement::ImbuementResult::Ok | imbuement::ImbuementResult::RevisionChanged
    ) {
        // Mutation replies carry no new slot snapshot. Never present the old quote as current.
        panel.window = None;
        panel.details = None;
        panel.details_pending = None;
        panel.details_attempted = false;
        panel.refresh_required = true;
    }
}

#[cfg(test)]
#[test]
fn catalogue_window_keeps_all_48_offers() -> Result<(), String> {
    let offers = (1..=26)
        .filter(|family| !matches!(family, 2 | 13))
        .flat_map(|family| {
            [2, 3].map(move |tier| imbuement::ImbuementOffer {
                family,
                tier,
                gold: if tier == 3 { 250_000 } else { 60_000 },
                requires_premium: true,
            })
        })
        .collect();
    let reply = imbuement::ImbuementReply {
        result: imbuement::ImbuementResult::Ok,
        window: Some(imbuement::ImbuementWindow {
            slot_count: 1,
            applied: vec![],
            offers,
            clear_gold: 15_000,
        }),
    };
    let encoded = imbuement::encode_imbuement_reply(&reply).map_err(|e| format!("{e:?}"))?;
    let response = ImbueResponse {
        status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
        reply: Some(imbuement::decode_imbuement_reply(&encoded).map_err(|e| format!("{e:?}"))?),
    };
    let mut panel = ImbuePanel::default();
    apply_result(&mut panel, &response);
    assert_eq!(panel.window.ok_or("window")?.offers.len(), 48);
    Ok(())
}
