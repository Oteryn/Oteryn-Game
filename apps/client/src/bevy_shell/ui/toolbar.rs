//! Gameplay toolbar: the button registry and what each button opens.
use super::{Control, ShellToolbar, Status};
use crate::bevy_shell::owner::HostOwner;
use bevy::prelude::*;
mod manager;
mod shortcuts;
pub(super) use manager::Manager;
pub(in crate::bevy_shell) use manager::project_manager;

// Label text doubles as the button caption; entries are ordered by label.
pub(super) const TOOLBAR: &[(&str, Control)] = &[
    ("Action Bars", Control::Shortcut("bars")),
    ("Analysers", Control::Analysers),
    ("Battle", Control::Panel("battle")),
    ("Bestiary", Control::Panel("bestiary")),
    ("Bestiary Tracker", Control::Shortcut("bestiary_tracker")),
    ("Bosstiary", Control::Panel("bosstiary")),
    ("Bosstiary Tracker", Control::Shortcut("bosstiary_tracker")),
    ("Chat", Control::Panel("chat")),
    ("Compendium", Control::Notices),
    ("Cyclopedia", Control::Shortcut("cyclopedia")),
    ("Cyclopedia Map", Control::CyclopediaMap),
    ("Daily Rewards", Control::Shortcut("rewards")),
    ("Depot", Control::Depot),
    ("Forge", Control::Shortcut("forge")),
    ("Highscores", Control::Shortcut("highscores")),
    ("Houses", Control::Shortcut("houses")),
    ("Imbuing", Control::Shortcut("imbuing")),
    ("Inspection", Control::Inspection),
    ("Inventory", Control::Panel("inventory")),
    ("Loot Containers", Control::Loot),
    ("Map Overview", Control::Shortcut("map")),
    ("Market", Control::Market),
    ("Minimap", Control::Panel("minimap")),
    ("NPC Trade", Control::Shortcut("npc")),
    ("Options", Control::Options),
    ("Outfit", Control::Shortcut("outfit")),
    ("Outfit Memorial", Control::Shortcut("memorial")),
    ("Party", Control::Party),
    ("Premium", Control::Shortcut("premium")),
    ("Prey", Control::Shortcut("prey")),
    ("Private Channels", Control::Shortcut("private")),
    ("Quest Log", Control::Quests),
    ("Report Bug", Control::Shortcut("bug")),
    ("Report Player", Control::Shortcut("report")),
    ("Rule Violation Record", Control::Shortcut("sanctions")),
    ("Skills", Control::Skills),
    ("Social", Control::Social),
    ("Spellbook", Control::Spells),
    ("Stash", Control::Shortcut("stash")),
    ("Store", Control::Shortcut("store")),
    ("Store Inbox", Control::StoreInbox),
    ("Task Board", Control::Shortcut("tasks")),
    ("Unjustified Points", Control::UnjustifiedPoints),
    ("VIP", Control::Panel("vip")),
    ("Weapon Proficiency", Control::Shortcut("proficiency")),
    ("Welcome Back", Control::Shortcut("returner")),
    ("Wheel", Control::Shortcut("wheel")),
];
pub(in crate::bevy_shell) fn open_toolbar(owner: &mut HostOwner, control: Control) -> bool {
    if owner.modal_active() {
        return false;
    }
    match control {
        Control::Options => owner.open_graphics(),
        Control::Shortcut(id) => return shortcuts::open(owner, id),
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
        Control::Panel("bestiary") => {
            if let Some(play) = &owner.play {
                play.link.charm_progression_open();
            }
        }
        Control::Panel(id) => {
            if let Some(play) = &owner.play {
                let mut preferences = owner.settings.preferences.clone();
                let visible =
                    preferences.panel_visible(id) || id == "vip" && play.link.vip_is_open();
                if visible {
                    preferences.visible_panels.retain(|panel| panel != id);
                } else {
                    preferences.visible_panels.push(id.into());
                }
                owner.settings.apply(preferences);
                if id == "bosstiary" && !visible {
                    play.link.boss_encounter_request(
                        oteryn_client_ui::boss_encounter::Intent::Catalogue,
                    );
                }
                if id == "vip" {
                    if visible {
                        play.link.vip_close();
                    } else {
                        play.link
                            .vip_request(oteryn_client_ui::vip::Intent::Query(0));
                    }
                }
            }
        }
        Control::CyclopediaMap => {
            if let Some(play) = &owner.play {
                play.link.cyclopedia_atlas_open();
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
        Control::Social => {
            if let Some(play) = &owner.play {
                play.link.discovery_open(232);
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
    children: Query<&Children>,
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
    let mut ordered = Vec::new();
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
            Some(control) if matches!(control, Control::ManageButtons) || TOOLBAR.iter().any(|entry| entry.1.id() == control.id())
        ) {
            if let (Some(control), Some(parent)) = (control, parent) {
                let rank = owner
                    .settings
                    .preferences
                    .toolbar_order
                    .as_ref()
                    .map_or_else(
                        || {
                            TOOLBAR
                                .iter()
                                .position(|entry| entry.1.id() == control.id())
                        },
                        |ids| ids.iter().position(|id| id == control.id()),
                    )
                    .unwrap_or(usize::MAX);
                ordered.push((rank, entity, parent.parent()));
            }
            let shown = control.is_some_and(|control| {
                matches!(control, Control::ManageButtons)
                    || owner
                        .settings
                        .preferences
                        .toolbar_order
                        .as_ref()
                        .is_none_or(|ids| ids.iter().any(|id| id == control.id()))
            });
            node.display =
                if (playing && shown) || (!playing && matches!(control, Some(Control::Options))) {
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
    ordered.sort_by_key(|entry| entry.0);
    if let Some((_, _, parent)) = ordered.first().copied()
        && children.get(parent).is_ok_and(|children| {
            !children
                .iter()
                .take(ordered.len())
                .eq(ordered.iter().map(|(_, entity, _)| *entity))
        })
    {
        for (index, (_, entity, _)) in ordered.iter().enumerate() {
            commands.entity(parent).insert_children(index, &[*entity]);
        }
    }
}

#[cfg(test)]
mod bestiary_tests {
    #[test]
    fn bestiary_is_reachable_from_gameplay_toolbar() -> Result<(), Box<dyn std::error::Error>> {
        let control = super::TOOLBAR
            .iter()
            .find(|(label, _)| *label == "Bestiary")
            .map(|(_, control)| *control)
            .ok_or("Bestiary toolbar entry missing")?;
        let mut owner = super::HostOwner::new(None)?;
        let (link, _, _) = crate::play::play_channel();
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        assert!(super::open_toolbar(&mut owner, control));
        assert!(
            owner
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .charm_progression_panel()
                .open
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidebar_toolbar_buttons_toggle_the_settings_owner() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut owner = HostOwner::new(None)?;
        let (link, _commands, _outbox) = crate::play::play_channel();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        });
        for id in ["battle", "chat", "inventory", "minimap", "vip"] {
            let mut preferences = owner.settings.preferences.clone();
            if !preferences.panel_visible(id) {
                preferences.visible_panels.push(id.into());
            }
            owner.settings.apply(preferences);
            assert!(open_toolbar(&mut owner, Control::Panel(id)));
            assert!(
                !owner.settings.preferences.panel_visible(id),
                "{id} must close"
            );
            assert!(open_toolbar(&mut owner, Control::Panel(id)));
            assert!(
                owner.settings.preferences.panel_visible(id),
                "{id} must reopen"
            );
        }
        Ok(())
    }
}
