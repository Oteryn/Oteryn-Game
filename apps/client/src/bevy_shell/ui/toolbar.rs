//! Gameplay toolbar: the button registry and what each button opens.
use super::{Control, ShellToolbar, Status};
use crate::bevy_shell::owner::HostOwner;
use bevy::prelude::*;

// Label text doubles as the button caption; entries are ordered by label.
pub(super) const TOOLBAR: &[(&str, Control)] = &[
    ("Analysers", Control::Analysers),
    ("Battle", Control::Panel("battle")),
    ("Chat", Control::Panel("chat")),
    ("Depot", Control::Depot),
    ("Compendium", Control::Notices),
    ("Inspection", Control::Inspection),
    ("Inventory", Control::Panel("inventory")),
    ("Loot Containers", Control::Loot),
    ("Market", Control::Market),
    ("Minimap", Control::Panel("minimap")),
    ("Options", Control::Options),
    ("Party", Control::Party),
    ("Quest Log", Control::Quests),
    ("Skills", Control::Skills),
    ("Spellbook", Control::Spells),
    ("Store Inbox", Control::StoreInbox),
    ("Unjustified Points", Control::UnjustifiedPoints),
    ("VIP", Control::Panel("vip")),
];
pub(super) fn open_toolbar(owner: &mut HostOwner, control: Control) -> bool {
    if owner.modal_active() {
        return false;
    }
    match control {
        Control::Options => owner.open_graphics(),
        Control::Depot => {
            if let Some(play) = &owner.play {
                play.link.open_depot();
            }
        }
        Control::Market => crate::bevy_shell::game_ui::open_market(owner),
        Control::Party => crate::bevy_shell::game_ui::open_party(owner),
        Control::Quests => crate::bevy_shell::game_ui::open_quests(owner),
        Control::UnjustifiedPoints => crate::bevy_shell::game_ui::unjustified_points::open(owner),
        Control::Spells => crate::bevy_shell::game_ui::open_spells(owner),
        Control::Panel(id) => {
            if let Some(play) = &owner.play {
                let mut preferences = owner.settings.preferences.clone();
                if !preferences.panel_visible(id) {
                    preferences.visible_panels.push(id.into());
                    owner.settings.apply(preferences);
                }
                if id == "vip" {
                    play.link
                        .vip_request(oteryn_client_ui::vip::Intent::Query(0));
                }
            }
        }
        Control::Analysers => {
            if let Some(play) = &owner.play {
                play.link.analyser_open();
            }
        }
        Control::Inspection => {
            if let Some(play) = &owner.play {
                play.link.inspection_set_open(true);
            }
        }
        Control::Loot => {
            if let Some(play) = &owner.play {
                play.link.discovery_open(98);
            }
        }
        Control::Notices => {
            if let Some(play) = &owner.play {
                play.link.notices_set_open(true);
            }
        }
        Control::StoreInbox => {
            if let Some(play) = &owner.play {
                play.link.store_inbox_set_open(true);
            }
        }
        Control::Skills => {
            if let Some(play) = &owner.play {
                play.link.skills_widget_request();
            }
        }
        _ => return false,
    }
    true
}

/// Keep admission controls outside the in-game HUD once play is installed.
// One disjoint query changes the toolbar, status and admission button nodes together.
#[allow(clippy::type_complexity)]
pub(in crate::bevy_shell) fn project_toolbar(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Query<Entity, With<crate::bevy_shell::game_ui::ToolbarRegion>>,
    mut nodes: Query<
        (
            Entity,
            &mut Node,
            Has<ShellToolbar>,
            Has<Status>,
            Option<&Control>,
            Option<&ChildOf>,
        ),
        Or<(With<ShellToolbar>, With<Status>, With<Control>)>,
    >,
) {
    let playing = owner.play.is_some();
    for (entity, mut node, toolbar, status, control, parent) in &mut nodes {
        if toolbar {
            if playing {
                if let Ok(region) = regions.single() {
                    if parent.is_none_or(|parent| parent.parent() != region) {
                        commands.entity(entity).insert(ChildOf(region));
                    }
                    node.position_type = PositionType::Relative;
                    node.width = percent(100.);
                    node.max_width = percent(100.);
                    node.overflow = Overflow::clip();
                    node.top = Val::Auto;
                    node.bottom = Val::Auto;
                    node.left = Val::Auto;
                    node.right = Val::Auto;
                }
            } else {
                if parent.is_some() {
                    commands.entity(entity).remove::<ChildOf>();
                }
                node.position_type = PositionType::Absolute;
                node.width = Val::Auto;
                node.max_width = Val::Auto;
                node.top = px(8.);
                node.bottom = Val::Auto;
                node.left = px(8.);
                node.right = px(8.);
            }
        } else if matches!(
            control,
            Some(control) if !matches!(control, Control::Options) && TOOLBAR.iter().any(|entry| entry.1.id() == control.id())
        ) {
            node.display = if playing {
                Display::Flex
            } else {
                Display::None
            };
        } else if status || matches!(control, Some(Control::Login | Control::CancelLogin)) {
            node.display = if playing {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
}
