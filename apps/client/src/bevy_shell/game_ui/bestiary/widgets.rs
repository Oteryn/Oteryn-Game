use super::*;
use oteryn_client_runtime::charm_progression::charm::CharmKind;
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    state: &State,
    panel: &Panel,
) -> Entity {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let body = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(760.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::axes(px(12.), px(24.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(body);
    source::dialog_caption(commands, body, "Bestiary and Charms", skins.font.clone());
    if let Some(kind) = state.confirmation {
        let message = match kind {
            Kind::Request(Intent::Unlock) => panel
                .reply
                .as_ref()
                .and_then(|r| {
                    r.charms
                        .charms
                        .iter()
                        .find(|c| c.charm.to_string() == panel.fields[0])
                })
                .map_or_else(
                    || "Unlock charm?".into(),
                    |c| {
                        let currency = match c.kind {
                            CharmKind::Major => "Charm Points",
                            CharmKind::Minor => "Minor Charm Echoes",
                        };
                        format!(
                            "Unlock charm {} for {} {currency}?",
                            c.charm, c.next_stage_cost
                        )
                    },
                ),
            _ => "Remove the assigned charm? The server charges the removal gold fee.".into(),
        };
        text(commands, skins, body, message);
        button(
            commands,
            skins,
            body,
            state,
            kind,
            "Confirm payment",
            true,
            panel,
        );
        button(
            commands,
            skins,
            body,
            state,
            Kind::Close,
            "Close",
            false,
            panel,
        );
        return overlay;
    }
    let scroll = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            Node {
                height: px(370.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(body).add_child(scroll);
    text(commands, skins, scroll, presentation(panel));
    for (id, name) in panel.charm_choices() {
        let status = panel
            .reply
            .as_ref()
            .and_then(|r| r.charms.charms.iter().find(|c| c.charm.get() == id))
            .map_or("", |c| {
                if c.unlocked_stage == 0 {
                    "Locked"
                } else {
                    "Unlocked"
                }
            });
        let name = format!("{name} — {status}");
        button(
            commands,
            skins,
            scroll,
            state,
            Kind::SelectCharm(id),
            &name,
            false,
            panel,
        );
    }
    for race in panel
        .reply
        .iter()
        .flat_map(|r| &r.bestiary)
        .map(|r| r.race.get())
        .chain(panel.catalogue.page.cards.iter().map(|c| u32::from(c.race)))
    {
        button(
            commands,
            skins,
            scroll,
            state,
            Kind::SelectRace(race),
            &format!("Select creature {race}"),
            false,
            panel,
        );
    }
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(body).add_child(footer);
    for (kind, label) in [
        (Kind::Request(Intent::Query), "Query progress"),
        (Kind::Catalogue(0), "Browse creatures"),
        (Kind::Catalogue(1), "Next page"),
        (Kind::Catalogue(4), "Previous class"),
        (Kind::Catalogue(5), "Next class"),
        (Kind::Catalogue(2), "Creature detail"),
        (Kind::Catalogue(3), "More loot"),
        (Kind::CharmNames, "Charm names"),
        (Kind::RaceNames, "Creature names"),
        (Kind::Request(Intent::Unlock), "Unlock"),
        (Kind::Request(Intent::Assign), "Assign"),
        (Kind::Request(Intent::Remove), "Remove"),
        (Kind::Close, "Close"),
    ] {
        button(commands, skins, footer, state, kind, label, false, panel);
    }
    overlay
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
}
// Carries both the displayed revision and generation to the observer.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    label: &str,
    confirm: bool,
    panel: &Panel,
) {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 138.),
        Control {
            generation: state.generation,
            revision: state.revision,
            kind,
            confirm,
        },
    ));
    if !panel.selected || panel.pending {
        entity.insert(bevy::ui::InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
