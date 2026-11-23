use super::*;
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    body: Entity,
    key: Key,
    panel: &WeaponProficiencyPanel,
    filters: &filters::Filters,
) {
    let list = pane(commands, skin, body, px(280.), 0);
    text(commands, skin, list, "Weapons");
    if let Some(catalogue) = &panel.catalogue {
        for (change, caption) in filters.controls() {
            button(
                commands,
                skin,
                list,
                Control {
                    key,
                    kind: Kind::Filter(change),
                },
                &caption,
                true,
            );
        }
        for row in panel
            .visible_weapons()
            .into_iter()
            .filter(|row| filters.rows.binary_search(row).is_ok())
        {
            let entry = &catalogue.entries[row];
            let caption = format!(
                "{}{} — Level {}\n{} / {} XP ({}%)",
                if panel.weapon == entry.definition {
                    "Selected: "
                } else {
                    ""
                },
                entry.name,
                progress_level(entry),
                entry.experience,
                mastery_target(entry),
                progress_percent(entry)
            );
            button(
                commands,
                skin,
                list,
                Control {
                    key,
                    kind: Kind::Weapon(row),
                },
                &caption,
                panel.selected && !panel.pending && entry.definition != 0,
            );
            let track = commands
                .spawn((
                    Node {
                        width: percent(100.),
                        height: px(8.),
                        flex_shrink: 0.,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                ))
                .id();
            let fill = commands
                .spawn((
                    Progress,
                    Node {
                        width: percent(progress_percent(entry) as f32),
                        height: percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.6, 0.3)),
                ))
                .id();
            commands.entity(track).add_child(fill);
            commands.entity(list).add_child(track);
        }
        text(
            commands,
            skin,
            list,
            &format!("{} / {} weapons", catalogue.entries.len(), catalogue.total),
        );
    } else {
        text(commands, skin, list, "No weapon catalogue received");
    }
}
