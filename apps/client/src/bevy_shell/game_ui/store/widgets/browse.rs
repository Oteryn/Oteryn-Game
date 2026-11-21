use super::*;

pub(super) fn controls(
    commands: &mut Commands,
    skin: &Skins,
    root: Entity,
    key: Key,
    panel: &StorePanel,
    fields: &[String; 4],
) {
    let ready = panel.selected && !panel.pending && panel.confirmed_purchase.is_none();
    field(
        commands,
        skin,
        root,
        3,
        "Search products (current category)",
        &fields[3],
    );
    let controls = row(commands, root);
    button(commands, skin, controls, key, Kind::Search, "Search", ready);
    button(
        commands,
        skin,
        controls,
        key,
        Kind::ClearSearch,
        "Clear",
        ready,
    );
    let sorts = [
        "Popularity",
        "Name A–Z",
        "Name Z–A",
        "Price low–high",
        "Price high–low",
    ];
    label(
        commands,
        skin,
        root,
        &format!(
            "Sort: {}",
            sorts
                .get(usize::from(panel.browser.sort))
                .copied()
                .unwrap_or("Popularity")
        ),
    );
    let controls = row(commands, root);
    for (sort, title) in sorts.iter().enumerate() {
        button(
            commands,
            skin,
            controls,
            key,
            Kind::Sort(sort as u8),
            title,
            ready,
        );
    }
}
