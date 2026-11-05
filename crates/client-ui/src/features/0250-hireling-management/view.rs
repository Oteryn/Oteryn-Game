use super::Panel;
pub fn panel_text(p: &Panel) -> String {
    let mut lines = vec!["Hireling identity and outfits".to_owned()];
    for e in &p.entries {
        let id = e
            .hireling
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        lines.push(format!(
            "{}: {} ({})\r\nOutfit {}; colors {:?}; services {}\r\n{}",
            e.name,
            id,
            if e.sex == 2 { "female" } else { "male" },
            e.look_type,
            e.colors,
            e.skills,
            e.position.map_or_else(
                || "In lamp".into(),
                |(x, y, z)| format!("Placed at {x}, {y}, {z}")
            )
        ));
    }
    if let Some(f) = p.feedback {
        lines.push(f.into());
    }
    if p.pending.is_some() {
        lines.push("Waiting for the server".into());
    }
    lines.join("\r\n")
}
