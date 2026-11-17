use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui::depot::{Intent, deposit_items};
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    state: &State,
    panel: &DepotPanel,
) -> Entity {
    Widgets {
        commands,
        skins,
        state,
    }
    .build(panel)
}
struct Widgets<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    skins: &'a Skins,
    state: &'a State,
}
impl Widgets<'_, '_, '_> {
    fn build(&mut self, panel: &DepotPanel) -> Entity {
        let overlay = self
            .commands
            .spawn((
                GameModal,
                crate::bevy_shell::ui::window_layout::WindowLayer("depot"),
                Name::new("Depot"),
                GlobalZIndex(165),
                TabGroup {
                    order: 3,
                    modal: true,
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.),
                    height: percent(100.),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ))
            .id();
        let root = self
            .commands
            .spawn((
                crate::bevy_shell::window_drag::DraggableDialog,
                source::frame(self.skins.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    width: px(640.),
                    max_width: percent(95.),
                    max_height: percent(95.),
                    padding: UiRect {
                        top: px(24.),
                        left: px(12.),
                        right: px(12.),
                        bottom: px(12.),
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6.),
                    ..default()
                },
            ))
            .id();
        self.commands.entity(overlay).add_child(root);
        source::dialog_caption(
            self.commands,
            root,
            "Depot — Ctrl+Shift+L",
            self.skins.font.clone(),
        );
        let enabled = panel.selected && !panel.pending;
        let boxes = self.line(root);
        for box_number in 1..=17 {
            self.button(
                boxes,
                Kind::Request(Intent::Browse {
                    box_number,
                    page: 0,
                }),
                &format!("Box {box_number}"),
                enabled,
            );
        }
        if let Some(reply) = &panel.reply {
            self.label(
                root,
                format!(
                    "Box {} · Page {} · {} items",
                    reply.box_number,
                    reply.page + 1,
                    reply.total
                ),
            );
            let navigation = self.line(root);
            for (caption, page, available) in [
                ("Previous", reply.page.saturating_sub(1), reply.page > 0),
                (
                    "Next",
                    reply.page.saturating_add(1),
                    (reply.page + 1) * 32 < reply.total,
                ),
                ("Refresh", reply.page, true),
            ] {
                self.button(
                    navigation,
                    Kind::Request(Intent::Browse {
                        box_number: reply.box_number,
                        page,
                    }),
                    caption,
                    enabled && available,
                );
            }
            let list = self.scroll(root);
            if reply.entries.is_empty()
                && matches!(
                    reply.outcome,
                    oteryn_client_ui::depot::DepotOutcome::Opened
                        | oteryn_client_ui::depot::DepotOutcome::Moved
                )
            {
                self.label(list, "This depot box is empty");
            }
            // Manual Sort Mode: with a deposit held, a row is the box slot it takes.
            let action = if panel.held.is_some() {
                "place here"
            } else {
                "withdraw stack"
            };
            for (index, item) in reply.entries.iter().enumerate() {
                self.button(
                    list,
                    Kind::Request(Intent::Withdraw(index)),
                    &format!("Item {} ×{} — {action}", item.definition, item.count),
                    enabled && reply.town.is_some(),
                );
            }
            self.label(root, "Carried items — deposit whole stack");
            let carried = self.scroll(root);
            for item in deposit_items(panel) {
                self.button(
                    carried,
                    Kind::Request(Intent::Deposit(item.handle)),
                    &format!(
                        "Item {} ×{} — {}",
                        item.item_definition_ref,
                        item.count,
                        if panel.held == Some(item.handle) {
                            "deposit at the first slot"
                        } else {
                            "deposit"
                        }
                    ),
                    enabled && reply.town.is_some(),
                );
            }
        } else {
            self.label(root, "Use a nearby depot locker to access your boxes.");
        }
        self.label(root, panel.feedback);
        self.button(root, Kind::Close, "Close (Esc)", true);
        overlay
    }
    fn label(&mut self, parent: Entity, value: impl Into<String>) {
        let entity = self
            .commands
            .spawn(source::text(
                value,
                source::font(self.skins.font.clone(), false),
            ))
            .id();
        self.commands.entity(parent).add_child(entity);
    }
    fn button(&mut self, parent: Entity, kind: Kind, caption: &str, enabled: bool) {
        let mut button = self.commands.spawn((
            source::button(
                self.skins.buttons.clone(),
                if matches!(kind, Kind::Request(Intent::Browse { .. })) {
                    80.
                } else {
                    145.
                },
            ),
            Control {
                generation: self.state.generation,
                revision: self.state.revision,
                kind,
            },
        ));
        if !enabled {
            button.insert(InteractionDisabled);
        }
        if matches!(
            kind,
            Kind::Request(Intent::Withdraw(_) | Intent::Deposit(_))
        ) {
            button.insert(Node {
                width: percent(100.),
                min_height: px(26.),
                height: Val::Auto,
                flex_shrink: 0.,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            });
        }
        let entity = button.observe(activate).id();
        let text = self
            .commands
            .spawn(source::button_caption(caption, self.skins.font.clone()))
            .id();
        self.commands.entity(entity).add_child(text);
        self.commands.entity(parent).add_child(entity);
    }
    fn line(&mut self, parent: Entity) -> Entity {
        let entity = self
            .commands
            .spawn(Node {
                column_gap: px(4.),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .id();
        self.commands.entity(parent).add_child(entity);
        entity
    }
    fn scroll(&mut self, parent: Entity) -> Entity {
        let entity = self
            .commands
            .spawn((
                ScrollArea,
                Node {
                    height: px(150.),
                    min_height: px(0.),
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3.),
                    ..default()
                },
            ))
            .id();
        self.commands.entity(parent).add_child(entity);
        entity
    }
}
