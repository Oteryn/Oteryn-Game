use super::*;
pub fn text(p: &WheelPanel) -> String {
    if !p.diagram.selected {
        return "This server does not offer the Wheel graph preview. Existing Wheel controls remain available.".into();
    }
    let mut out = String::new();
    let spent = p.points.iter().map(|v| u32::from(*v)).sum::<u32>();
    let Some(r) = metadata(p) else {
        return format!(
            "Refresh the current Wheel to load its actual graph and perks.\n{}",
            p.diagram.feedback
        );
    };
    out.push_str(&format!("Available points: {} · ring-unlock points: {}\nDraft allocation: {spent} · {}\nRemoving points/reset: {}\nAllocation eligibility: {}\n",r.available,r.unlock_points,if spent<=r.available{format!("{} remaining",r.available-spent)}else{format!("{} over budget",spent-r.available)},if r.removal_permitted{"currently permitted by combat-lock owner"}else{"currently blocked by combat-lock owner"},if r.eligible{"eligible"}else{"not eligible under current server rules"}));
    if let Some(n) = r.nodes.get(p.slot) {
        out.push_str(&format!("\nSlot {} · {} / {}\n{}\nMinimum ring-unlock budget: {}\nUnlock from a full open slot: {}\nOne point configured contribution:\n{}\nAt full slot:\n{}\n",p.slot+1,p.points[p.slot],n.capacity,n.name,n.minimum_points,if n.unlock_from.is_empty(){"centre slot".into()}else{n.unlock_from.iter().map(|i|(u16::from(*i)+1).to_string()).collect::<Vec<_>>().join(", ")},n.per_point.join("\n"),n.full_slot.join("\n")));
        if super::super::is_vessel_node(&n.name) {
            out.push_str(&format!("{}\n", super::super::vessel_node_perk(1)));
        }
        if let Some(perk) = r.revelations.get(usize::from(n.domain)) {
            out.push_str(&format!("\n{} · domain revelations\n", perk.name));
            for (stage, s) in perk.stages.iter().enumerate() {
                out.push_str(&format!(
                    "Stage {} at {} domain points:\n{}\n",
                    stage + 1,
                    s.minimum_points,
                    s.effects.join("\n")
                ));
            }
        }
    }
    if let Some(r) = current(p) {
        out.push_str(&format!("\nCurrent allocation configured effects:\n{}\nDraft configured effects:\n{}\nDraft validation: {}\n",r.current.join("\n"),r.draft.join("\n"),if r.draft_valid{"valid"}else{&r.reason}));
    } else {
        out.push_str("\nDraft changed. Requesting a fresh server preview before submission.\n");
    }
    out.push_str(&format!("\n{}\n{}\nConfigured effects are previews; combat and spells recheck their live eligibility and conditions. The writer rechecks its action interval before applying a change.\nCatalogue: {}",if p.diagram.reset_requested{"Reset draft: Apply reset sends the original reset operation."}else{"Point edits remain local until Save draft is confirmed."},p.diagram.feedback,r.catalogue));
    out
}
