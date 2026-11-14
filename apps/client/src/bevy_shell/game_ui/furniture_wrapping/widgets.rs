use super::*;
use bevy::{input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea};
use oteryn_client_ui_bevy as source;
pub(super) fn build(commands: &mut Commands, skins: &Skins, key: Key, panel: &Panel) -> Entity {
    let root = commands.spawn((GameModal, GlobalZIndex(164), TabGroup { order: 2, modal: true },
        Name::new("furniture_wrapping.window"), Node {
            position_type: PositionType::Absolute, width: percent(100.), height: percent(100.),
            justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default()
        }, BackgroundColor(Color::srgba(0.,0.,0.,0.45)))).id();
    let body = commands.spawn((source::frame(skins.frame.clone(), [4.,17.,4.,4.]),
        crate::bevy_shell::window_drag::DraggableDialog, Node {
            width: px(560.), max_width: percent(95.), max_height: percent(95.),
            padding: UiRect { top: px(24.), left: px(12.), right: px(12.), bottom: px(10.) },
            flex_direction: FlexDirection::Column, row_gap: px(8.), ..default()
        })).id();
    commands.entity(root).add_child(body);
    source::dialog_caption(commands, body, "House furniture", skins.font.clone());
    let text = commands.spawn((source::text(&oteryn_client_ui::house_furniture_wrap::panel_text(panel), source::font(skins.font.clone(), false)),
        Name::new("furniture_wrapping.catalogue"))).id();
    let list = commands.spawn((ScrollArea, source::frame(skins.inset.clone(), [1.;4]), Node {
        max_height: px(300.), overflow: Overflow::scroll_y(), flex_direction: FlexDirection::Column, padding: UiRect::all(px(5.)), ..default()
    })).id();
    commands.entity(body).add_child(list);
    commands.entity(list).add_child(text);
    if let Some(catalogue) = &panel.catalogue {
        for (index, _) in catalogue.entries.iter().enumerate() {
            let number = (index + 1).to_string();
            let caption = if panel.choice == number { format!("Selected: {number}") } else { format!("Select {number}") };
            button(commands, skins, body, Control { key, kind: Kind::Select(index) }, &caption, panel.selected && !panel.pending);
        }
    }
    for (intent, caption) in [(Intent::Refresh, "Refresh"), (Intent::Previous, "Previous"), (Intent::Next, "Next"), (Intent::Apply, "Wrap / place")] {
        let caption = if intent == Intent::Apply && panel.reviewed_payload.is_some() { "Confirm Wrap / place" } else { caption };
        button(commands, skins, body, Control { key, kind: Kind::Request(intent) }, caption, panel.selected && !panel.pending);
    }
    button(commands, skins, body, Control { key, kind: Kind::Close }, "Close", true);
    root
}
fn button(commands: &mut Commands, skins: &Skins, parent: Entity, control: Control, caption: &str, enabled: bool) {
    let mut button = commands.spawn((source::button(skins.buttons.clone(), 160.), control));
    if !enabled { button.insert(InteractionDisabled); }
    let entity = button.observe(activate).id();
    let label = commands.spawn(source::button_caption(caption, skins.font.clone())).id();
    commands.entity(entity).add_child(label);
    commands.entity(parent).add_child(entity);
}
