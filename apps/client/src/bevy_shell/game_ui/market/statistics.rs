//! Completed-price ranges share a zero-based gold scale; these are not time-series data.
use super::*;

pub(super) fn render(commands: &mut Commands, skin: &Skins, parent: Entity, panel: &MarketPanel) {
    let Some(stats) = panel
        .details
        .statistics
        .as_ref()
        .filter(|s| s.definition == panel.definition)
    else {
        return;
    };
    let ceiling = stats
        .buy
        .maximum
        .into_iter()
        .chain(stats.sell.maximum)
        .max();
    let Some(ceiling) = ceiling.filter(|price| *price > 0) else {
        widgets::label(commands, skin, parent, "No completed prices to chart");
        return;
    };
    widgets::label(
        commands,
        skin,
        parent,
        &format!(
            "30-day completed price ranges · 0–{ceiling} gold · line: min–max · marker: average"
        ),
    );
    for (title, summary, color) in [
        ("Buy offers", &stats.buy, Color::srgb(0.35, 0.7, 0.95)),
        ("Sell offers", &stats.sell, Color::srgb(0.95, 0.7, 0.3)),
    ] {
        let (Some(min), Some(max), Some(average)) =
            (summary.minimum, summary.maximum, summary.average)
        else {
            widgets::label(
                commands,
                skin,
                parent,
                &format!("{title}: no completed trades"),
            );
            continue;
        };
        widgets::label(
            commands,
            skin,
            parent,
            &format!(
                "{title}: {min}–{max} gold · average {average} · {} trades · {} units",
                summary.trades, summary.units
            ),
        );
        let track = commands
            .spawn((
                Node {
                    width: percent(100.),
                    height: px(14.),
                    flex_shrink: 0.,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
            ))
            .id();
        commands.entity(parent).add_child(track);
        let position = |price: u64| percent((price as f64 / ceiling as f64 * 100.) as f32);
        let range = commands
            .spawn((
                Name::new("market.price.range"),
                Node {
                    position_type: PositionType::Absolute,
                    left: position(min),
                    width: position(max - min),
                    min_width: px(2.),
                    top: px(5.),
                    height: px(4.),
                    ..default()
                },
                BackgroundColor(color),
            ))
            .id();
        let marker = commands
            .spawn((
                Name::new("market.price.average"),
                Node {
                    position_type: PositionType::Absolute,
                    left: position(average),
                    // Center the marker, including when it lies at the right edge.
                    margin: UiRect::left(px(-1.)),
                    width: px(2.),
                    height: px(14.),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ))
            .id();
        commands.entity(track).add_children(&[range, marker]);
    }
}
