//! Reference effect descriptions are advisory; every result value comes from the retained receipt.
use super::{ItemForgePanel, reply, resource_dialog::Content};
use oteryn_client_runtime::{cui3::item_forge::Request, forge_preview::Effects};
pub fn effects_text(e: &Effects) -> String {
    let mut text = format!(
        "{}\nCurrent tier: {} · classification {} · maximum tier {}\n\nReference tier effects (server inspection values; not a claim of active combat effects):",
        e.item.name, e.item.tier, e.item.classification, e.item.maximum_tier
    );
    for description in &e.tiers {
        text.push_str(&format!("\nTier {description}"));
    }
    text.push_str("\n\nForge eligibility and costs require a current pair preview. The server validates the operation again.");
    text
}
pub fn show_result(p: &mut ItemForgePanel, attempt: Request, receipt: reply::Receipt) {
    let lookup = |handle| {
        p.tier_effects
            .iter()
            .find(|e| Some(e.item.handle) == handle)
            .cloned()
    };
    let target = lookup(attempt.first_item);
    let donor = lookup(attempt.second_item);
    p.resource_dialog.reset();
    p.resource_dialog.content = Some(Content::Exaltation {
        receipt,
        target,
        donor,
    });
}
fn tier_text(label: &str, tier: Option<u8>, e: Option<&Effects>) -> String {
    match tier {
        None => format!("{label}: consumed"),
        Some(tier) => match e.and_then(|e| e.tiers.get(usize::from(tier))) {
            Some(description) => format!("{label}: tier {description} (reference effect)"),
            None => format!("{label}: tier {tier} · effect description unavailable"),
        },
    }
}
pub fn result_text(
    r: &reply::Receipt,
    target: Option<&Effects>,
    donor: Option<&Effects>,
) -> String {
    let mut text = format!(
        "{}\n{}",
        super::resource_dialog::operation_name(r.operation),
        if !r.applied {
            "Not applied"
        } else if r.succeeded {
            "Succeeded"
        } else {
            "Attempt applied; fusion failed"
        }
    );
    if let Some(reason) = &r.reason {
        text.push_str(&format!("\nServer reason: {reason}"));
    }
    if r.applied {
        text.push_str(&format!(
            "\n{}",
            match r.bonus {
                0 => "No additional bonus",
                1 => "Dust cost waived",
                2 => "Core cost waived",
                3 => "Gold cost waived",
                4 => "Donor retained one tier lower",
                5 => "Donor retained at its original tier",
                6 => "Donor retained at the normal result tier",
                7 =>
                    "Extra target tier bonus; the final tier below includes the server's class cap",
                8 => "Failure protection preserved the donor tier",
                _ => "Unrecognized result bonus",
            }
        ));
        if let Some(a) = &r.resources {
            text.push_str(&format!(
                "\n{}\n{}\nActually spent: {} gold, {} cores\nDust after: {} / {}",
                tier_text("Target", a.first_tier, target),
                tier_text("Donor", a.second_tier, donor),
                a.gold_spent,
                a.cores_spent,
                a.dust,
                a.dust_limit
            ));
        }
    }
    text
}
#[cfg(test)]
#[path = "exaltation_tests.rs"]
mod tests;
