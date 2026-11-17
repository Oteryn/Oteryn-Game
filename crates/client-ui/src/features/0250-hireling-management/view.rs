use super::Panel;
pub fn panel_text(p: &Panel) -> String {
    let mut lines = vec![tr!("Hireling identity and outfits").to_owned()];
    for e in &p.entries {
        let id = e
            .hireling
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let jobs = [tr!("Banker"), tr!("Cook"), tr!("Steward"), tr!("Trader")]
            .iter()
            .enumerate()
            .filter(|(bit, _)| e.skills & (1 << bit) != 0)
            .map(|(_, name)| *name)
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(lfmt!(
            "Jobs: {}",
            if jobs.is_empty() { tr!("None") } else { &jobs }
        ));
        if let Some(super::wire::Request::Outfit {
            hireling,
            look_type,
            colors,
        }) = &p.pending
            && *hireling == e.hireling
        {
            lines.push(lfmt!(
                "Preview: {} ({}), outfit {look_type}, colors {colors:?}",
                e.name,
                if e.sex == 2 { "female" } else { "male" },
                colors = colors,
                look_type = look_type
            ));
        }
        lines.push(lfmt!(
            "{}: {} ({})\r\nOutfit {}; colors {:?}; services {}\r\n{}",
            e.name,
            id,
            if e.sex == 2 { "female" } else { "male" },
            e.look_type,
            e.colors,
            e.skills,
            e.position.map_or_else(
                || tr!("In lamp").into(),
                |(x, y, z)| lfmt!("Placed at {x}, {y}, {z}", x = x, y = y, z = z)
            )
        ));
    }
    if let Some(f) = p.feedback {
        lines.push(f.into());
    }
    if p.pending.is_some() {
        lines.push(tr!("Waiting for the server").into());
    }
    lines.join("\r\n")
}
