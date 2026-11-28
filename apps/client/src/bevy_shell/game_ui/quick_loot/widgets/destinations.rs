use super::*;
use crate::play::feature_0098_quick_loot::view::quick_loot_pouch_route;

pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    destinations: Entity,
    key: Key,
    panel: &Panel,
    (pouch, store_selected, inbox_has_next): (Option<[u8; 16]>, bool, bool),
) {
    text(commands, skins, destinations, "Loot containers");
    if panel.destinations_selected {
        button(
            commands,
            skins,
            destinations,
            key,
            Kind::Request(Action::ReviewDestinations),
            "Refresh destinations",
            165.,
            panel,
        );
    }
    if panel.destinations_selected && store_selected && pouch.is_none() {
        let kind = if inbox_has_next {
            Kind::NextInboxPage
        } else {
            Kind::RefreshInbox
        };
        let label = if inbox_has_next {
            "Next Store Inbox page"
        } else {
            "Check Store Inbox"
        };
        button(commands, skins, destinations, key, kind, label, 190., panel);
    }
    for category in std::iter::once(None).chain(Category::ALL.into_iter().map(Some)) {
        if !panel.destinations_selected
            && category.is_some_and(|category| category as u8 > Category::Other as u8)
        {
            continue;
        }
        let label = match category {
            None => "Unassigned".into(),
            Some(Category::CreatureProduct) => "Creature products".into(),
            Some(Category::DistanceWeapon) => "Distance weapons".into(),
            Some(Category::FistWeapon) => "Fist weapons".into(),
            Some(Category::Armor) => "Armors".into(),
            Some(category) => format!("{category:?}"),
        };
        let row = column(commands, destinations);
        text(commands, skins, row, label);
        let confirmed = panel
            .destinations
            .iter()
            .find(|action| matches!(action,Action::Destination {category:c,..} if *c==category));
        if let Some(Action::Destination {
            recipient,
            fallback,
            ..
        }) = confirmed
        {
            let recipient = recipient_name(*recipient);
            text(
                commands,
                skins,
                row,
                format!(
                    "Confirmed: {recipient}; fallback {}",
                    if *fallback { "on" } else { "off" }
                ),
            );
        } else {
            text(commands, skins, row, "Saved destination not available");
        }
        if let Some(obtain) = panel
            .obtain_destinations
            .iter()
            .find(|destination| destination.category == category)
        {
            let recipient = recipient_name(obtain.recipient);
            text(commands, skins, row, format!("Obtain: {recipient}"));
        }
        let buttons = row_node(commands, row);
        if matches!(
            confirmed,
            Some(Action::Destination {
                recipient: Recipient::MainContainer,
                ..
            })
        ) {
            button(
                commands,
                skins,
                buttons,
                key,
                Kind::OpenDestination(category),
                "Open",
                60.,
                panel,
            );
        }
        for (fallback, label) in [(false, "Use main"), (true, "Main + fallback")] {
            button(
                commands,
                skins,
                buttons,
                key,
                Kind::Request(Action::Destination {
                    category,
                    recipient: Recipient::MainContainer,
                    id: None,
                    fallback,
                }),
                label,
                102.,
                panel,
            );
        }
        for (fallback, label) in [(false, "Use gold pouch"), (true, "Gold pouch + fallback")] {
            if let Some(assignment) = quick_loot_pouch_route(panel, pouch, category, fallback)
                && confirmed != Some(&assignment)
            {
                button(
                    commands,
                    skins,
                    row,
                    key,
                    Kind::Request(assignment),
                    label,
                    175.,
                    panel,
                );
            }
        }
        if let Some(Action::Destination {
            category,
            recipient,
            id,
            fallback,
        }) = confirmed
        {
            button(
                commands,
                skins,
                row,
                key,
                Kind::Request(Action::Destination {
                    category: *category,
                    recipient: *recipient,
                    id: *id,
                    fallback: !*fallback,
                }),
                "Toggle confirmed fallback",
                190.,
                panel,
            );
        }
    }
}

fn recipient_name(recipient: Recipient) -> &'static str {
    match recipient {
        Recipient::MainContainer => "Main backpack",
        Recipient::Container => "Container",
        Recipient::GoldPouch => "Gold pouch",
    }
}
