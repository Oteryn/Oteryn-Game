use super::*;
pub fn balances(p: &WheelPanel) -> String {
    p.confirmed_resources.map_or_else(||"Live resource balances unavailable on this server; costs remain server-confirmed.".into(),|r|format!("Available: {} carried gold; {} bank gold\nUnrevealed gems: {} lesser / {} regular / {} greater\nFragments: {} basic / {} supreme",r.carried_gold,r.bank_gold,r.lesser_gems,r.regular_gems,r.greater_gems,r.basic_fragments,r.supreme_fragments))
}
fn number(value: i64) -> String {
    let sign = if value < 0 { "−" } else { "+" };
    let abs = value.unsigned_abs();
    let mut out = format!("{sign}{}", abs / 1000);
    if !abs.is_multiple_of(1000) {
        out.push_str(format!(".{:03}", abs % 1000).trim_end_matches('0'));
    }
    out
}
pub fn modifier(p: &WheelPanel, kind: u8, id: u16) -> String {
    let Some(row) =
        current(p).and_then(|r| r.modifiers.iter().find(|m| m.kind == kind && m.id == id))
    else {
        return format!(
            "{}\nExecuted values unavailable; refresh Gem details.",
            super::super::modifier_label(p, kind, id)
        );
    };
    let mut text = format!(
        "{} · stored shared grade {}\n",
        row.name,
        p.grade(kind, id) + 1
    );
    let qualified = current(p).is_some_and(|r| r.grades_current);
    if !qualified {
        text.push_str("Stored grades belong to an earlier catalogue; current rules use Grade I.\n");
    }
    for e in &row.effects {
        text.push_str(&format!(
            "{}{}\n",
            e.name.replace('_', " "),
            if e.target.is_empty() {
                String::new()
            } else {
                format!(" · {}", e.target)
            }
        ));
        for (grade, v) in e.values.iter().enumerate() {
            text.push_str(&format!(
                "  Grade {}: {}{}{}\n",
                grade + 1,
                number(*v),
                match e.unit {
                    0 => " percentage points",
                    1 => "",
                    2 => " seconds",
                    _ => " points",
                },
                if grade == usize::from(if qualified { p.grade(kind, id) } else { 0 }) {
                    " ← current"
                } else {
                    ""
                }
            ));
        }
    }
    text
}
pub fn details(p: &WheelPanel) -> String {
    if let Some(review) = &p.gems.review {
        return review.text.clone();
    }
    let mut text = balances(p);
    if let Some(gem) = p.selected_gem() {
        if current(p).is_some_and(|r| !r.qualified_gems.contains(&gem.id)) {
            text.push_str("\nThis gem belongs to an earlier catalogue; the values below are current comparisons, not active gem effects.\n");
        }
        text.push_str(&format!("\n\n{}\n", super::super::gem_label(gem)));
        for (position, id) in gem.modifiers.iter().enumerate() {
            text.push_str(&format!(
                "\nModifier position {} (activation depends on current vessel resonance):\n{}",
                position + 1,
                modifier(p, u8::from(position == 2), *id)
            ));
        }
    }
    if let Some((kind, id)) = p.modifier {
        text.push_str(&format!(
            "\n\nWorkshop comparison:\n{}",
            modifier(p, kind, id)
        ));
    }
    if let Some(reply) = current(p) {
        text.push_str(&format!("\nServer values for {}\nValues describe a modifier at each shared grade; only active socket positions contribute to combat.",reply.vocation));
    }
    if let Some(feedback) = &p.gems.feedback {
        text.push_str(&format!("\n{feedback}"));
    }
    if let Some(feedback) = p.feedback {
        text.push_str(&format!("\n{feedback}"));
    }
    text
}
