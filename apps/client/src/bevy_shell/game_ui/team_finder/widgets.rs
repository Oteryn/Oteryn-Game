use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::{InteractionDisabled, ScrollPosition},
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::team_finder::{MemberStatus, TeamFinderResult};
use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
use oteryn_client_ui_bevy as source;

mod controls;
use controls::{button, buttons, caption, input, label, row};

pub(super) const LISTING_FIELDS: usize = 9;

/// The leader listing editor captions, in the field order the session parses.
fn listing_captions() -> [fn() -> &'static str; LISTING_FIELDS] {
    [
        || tr!("Minimum level"),
        || tr!("Maximum level"),
        || tr!("Vocation mask (1-255, at least one vocation)"),
        || tr!("Team size"),
        || tr!("Free slots"),
        || tr!("Party members join (0 or 1)"),
        || tr!("Activity (1 boss, 2 hunt, 3 quest)"),
        || tr!("Boss, hunt or quest id"),
        || tr!("Hunting area (hunts only)"),
    ]
}

#[derive(Component)]
pub(super) struct Caption(fn() -> &'static str);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tab {
    Join,
    Manage,
}

impl Tab {
    pub(super) fn for_panel(panel: &Panel) -> Self {
        if panel.applicants {
            Self::Manage
        } else {
            Self::Join
        }
    }
}

struct Button {
    entity: Entity,
    text: Entity,
}

impl Button {
    fn project(&self, commands: &mut Commands, key: Key, kind: Kind, value: &str, enabled: bool) {
        let mut entity = commands.entity(self.entity);
        entity.insert((
            Control { key, kind },
            Name::new(format!("Team Finder: {value}")),
        ));
        if enabled {
            entity.remove::<InteractionDisabled>();
        } else {
            entity.insert(InteractionDisabled);
        }
        commands.entity(self.text).insert(Text::new(value));
    }
}

struct Page {
    entity: Entity,
    list: Entity,
    status: Entity,
    rows: Vec<Button>,
    listing_handles: Vec<u64>,
    feedback: Entity,
    buttons: [Button; 8],
}

pub(super) struct Widgets {
    header: Entity,
    join: Page,
    manage: Page,
    leader_feedback: Entity,
    close: Button,
}

pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    search: [&str; 2],
    listing: &[String; LISTING_FIELDS],
) -> (Entity, Widgets) {
    let root = commands
        .spawn((
            GameModal,
            crate::bevy_shell::ui::window_layout::WindowLayer("team_finder"),
            GlobalZIndex(165),
            TabGroup {
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
        ))
        .id();
    let dialog = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(720.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::all(px(12.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(dialog);

    let header = source::dialog_caption(commands, dialog, "", skins.font.clone());
    let join = page_root(commands, dialog, Tab::Join);
    let join_tabs = row(commands, join);
    let join_switch = button(commands, skins, join_tabs, false);
    caption(commands, skins, join, || {
        tr!(
            "Search this page by leader name and level (blank = any level). Filters apply to each page."
        )
    });
    for (field, value) in [Field::Name, Field::Level].into_iter().zip(search) {
        input(commands, skins, join, field, value.to_owned());
    }
    let filters = row(commands, join);
    let [search, capacity, activity] = buttons(commands, skins, filters);
    let (list, status) = result_list(commands, skins, join, 300.);
    let feedback = label(commands, skins, join, "");
    let footer = row(commands, join);
    let [refresh, next, apply, cancel] = buttons(commands, skins, footer);
    let join = Page {
        entity: join,
        list,
        status,
        feedback,
        rows: Vec::new(),
        listing_handles: Vec::new(),
        buttons: [
            join_switch,
            search,
            capacity,
            activity,
            refresh,
            next,
            apply,
            cancel,
        ],
    };

    let manage = page_root(commands, dialog, Tab::Manage);
    let manage_tabs = row(commands, manage);
    let join_tab = button(commands, skins, manage_tabs, false);
    caption(commands, skins, manage, || {
        tr!("Players who applied to or were invited to your team. Select one to decide.")
    });
    let (list, status) = result_list(commands, skins, manage, 180.);
    let feedback = label(commands, skins, manage, "");
    let decisions = row(commands, manage);
    let [refresh, next, invite, accept, decline] = buttons(commands, skins, decisions);
    caption(commands, skins, manage, || tr!("Your team listing"));
    for (index, value) in listing_captions().into_iter().enumerate() {
        let line = row(commands, manage);
        caption(commands, skins, line, value);
        input(
            commands,
            skins,
            line,
            Field::Listing(index),
            listing[index].clone(),
        );
    }
    let leader_feedback = label(commands, skins, manage, "");
    let footer = row(commands, manage);
    let [publish, withdraw] = buttons(commands, skins, footer);
    let manage = Page {
        entity: manage,
        list,
        status,
        feedback,
        rows: Vec::new(),
        listing_handles: Vec::new(),
        buttons: [
            join_tab, refresh, next, invite, accept, decline, publish, withdraw,
        ],
    };

    let close = button(commands, skins, dialog, false);
    (
        root,
        Widgets {
            header,
            join,
            manage,
            leader_feedback,
            close,
        },
    )
}

fn page_root(commands: &mut Commands, dialog: Entity, tab: Tab) -> Entity {
    let page = commands
        .spawn((
            tab,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                min_height: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(dialog).add_child(page);
    page
}

fn result_list(
    commands: &mut Commands,
    skins: &Skins,
    page: Entity,
    height: f32,
) -> (Entity, Entity) {
    let list = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(height),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(page).add_child(list);
    (list, label(commands, skins, list, ""))
}

fn visibility(commands: &mut Commands, entity: Entity, visible: bool) {
    commands
        .entity(entity)
        .queue(move |mut entity: EntityWorldMut| {
            if let Some(mut node) = entity.get_mut::<Node>() {
                node.display = if visible {
                    Display::Flex
                } else {
                    Display::None
                };
            }
            entity.insert(if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        });
}

impl Page {
    fn project(
        &mut self,
        commands: &mut Commands,
        skins: &Skins,
        key: Key,
        panel: &Panel,
        tab: Tab,
    ) {
        let visible = Tab::for_panel(panel) == tab;
        visibility(commands, self.entity, visible);
        if !visible {
            return;
        }

        // A pending refresh temporarily clears the result. Compare completed pages.
        if let Some(result) = &panel.result {
            let handles: Vec<_> = match result {
                TeamFinderResult::Browse { listings, .. } => {
                    listings.iter().map(|row| row.leader_handle).collect()
                }
                TeamFinderResult::Applicants { members, .. } => {
                    members.iter().map(|row| row.handle).collect()
                }
            };
            if self.listing_handles != handles {
                commands.entity(self.list).insert(ScrollPosition::default());
                self.listing_handles = handles;
            }
        }

        let names = panel.visible_names();
        let status = if !panel.selected {
            tr!("Team discovery is unavailable in this session")
        } else if panel.result.is_none() && panel.pending.is_none() {
            match tab {
                Tab::Join => tr!("Refresh to search for teams."),
                Tab::Manage => tr!("Refresh to see your applicants."),
            }
        } else if panel.result.is_some() && names.is_empty() {
            match tab {
                Tab::Join => {
                    tr!("No teams match on this page. Adjust filters or browse the next page.")
                }
                Tab::Manage => tr!("No applicants on this page."),
            }
        } else {
            ""
        };
        commands.entity(self.status).insert(Text::new(status));
        visibility(commands, self.status, !status.is_empty());
        commands
            .entity(self.feedback)
            .insert(Text::new(panel.feedback.clone()));

        // Grow with the page, then reuse the same row controls for later replies.
        while self.rows.len() < names.len() {
            self.rows.push(button(commands, skins, self.list, true));
        }
        for (index, row) in self.rows.iter().enumerate() {
            let name = names.get(index);
            visibility(commands, row.entity, name.is_some());
            let value = match name {
                Some(name) if index == panel.selected_row => format!("> {name}"),
                Some(name) => name.clone(),
                None => String::new(),
            };
            row.project(
                commands,
                key,
                Kind::Select(index),
                &value,
                name.is_some() && panel.selected && panel.pending.is_none(),
            );
        }
    }
}

pub(super) fn captions(mut captions: Query<(&Caption, &mut Text)>) {
    for (caption, mut text) in &mut captions {
        let value = (caption.0)();
        if text.0 != value {
            text.0 = value.into();
        }
    }
}

impl Widgets {
    pub(super) fn project(
        &mut self,
        commands: &mut Commands,
        skins: &Skins,
        key: Key,
        snapshot: &Snapshot,
    ) {
        let (_, panel, leader) = snapshot;
        let title = if panel.applicants {
            tr!("Social — Manage Team")
        } else {
            tr!("Social — Join Team")
        };
        commands
            .entity(self.header)
            .queue(move |header: EntityWorldMut| {
                if let Some(text) = header
                    .get::<Children>()
                    .and_then(|children| children.first())
                    .copied()
                {
                    header
                        .into_world_mut()
                        .entity_mut(text)
                        .insert(Text::new(title));
                }
            });
        self.join.project(commands, skins, key, panel, Tab::Join);
        self.manage
            .project(commands, skins, key, panel, Tab::Manage);

        let ready = panel.selected && panel.pending.is_none();
        if panel.applicants {
            let [
                join,
                refresh,
                next,
                invite,
                accept,
                decline,
                publish,
                withdraw,
            ] = &self.manage.buttons;
            let chosen = ready && panel.handle().is_some();
            let editable = leader.selected && !leader.pending;
            for (button, kind, caption, enabled) in [
                (join, Kind::Browse(false), tr!("Join Team"), ready),
                (refresh, Kind::Applicants(false), tr!("Refresh"), ready),
                (
                    next,
                    Kind::Applicants(true),
                    tr!("Next page"),
                    ready && panel.next().is_some(),
                ),
                (invite, Kind::Decide(2), tr!("Invite"), chosen),
                (accept, Kind::Decide(3), tr!("Accept"), chosen),
                (decline, Kind::Decide(4), tr!("Decline"), chosen),
                (
                    publish,
                    Kind::Publish(Vec::new()),
                    tr!("Publish listing"),
                    editable,
                ),
                (
                    withdraw,
                    Kind::Withdraw,
                    tr!("Remove listing"),
                    editable && leader.confirmed.is_some(),
                ),
            ] {
                button.project(commands, key, kind, caption, enabled);
            }
        } else {
            let [
                manage,
                search,
                capacity,
                activity,
                refresh,
                next,
                apply,
                cancel,
            ] = &self.join.buttons;
            let mut open = panel.filter.clone();
            open.only_open = !open.only_open;
            let mut target = panel.filter.clone();
            target.target = (target.target.min(3) + 1) % 4;
            let selected = match &panel.result {
                Some(TeamFinderResult::Browse { listings, .. }) => panel
                    .visible_indices()
                    .get(panel.selected_row)
                    .and_then(|index| listings.get(*index)),
                _ => None,
            };
            let can_apply = selected.is_some_and(|row| {
                matches!(row.status, MemberStatus::None | MemberStatus::Denied)
                    && row.occupied_slots < row.listing.slots
            });
            let can_cancel = selected.is_some_and(|row| {
                matches!(
                    row.status,
                    MemberStatus::Pending | MemberStatus::Invited | MemberStatus::Accepted
                )
            });
            for (button, kind, caption, enabled) in [
                (manage, Kind::Applicants(false), tr!("Manage Team"), ready),
                (
                    search,
                    Kind::Filter(panel.filter.clone()),
                    tr!("Search"),
                    ready,
                ),
                (
                    capacity,
                    Kind::Filter(open),
                    if panel.filter.only_open {
                        tr!("Open slots only")
                    } else {
                        tr!("All capacities")
                    },
                    ready,
                ),
                (
                    activity,
                    Kind::Filter(target),
                    [
                        tr!("All activities"),
                        tr!("Boss"),
                        tr!("Hunt"),
                        tr!("Quest"),
                    ][usize::from(panel.filter.target.min(3))],
                    ready,
                ),
                (refresh, Kind::Browse(false), tr!("Refresh"), ready),
                (
                    next,
                    Kind::Browse(true),
                    tr!("Next page"),
                    ready && panel.next().is_some(),
                ),
                (
                    apply,
                    Kind::Apply(false),
                    tr!("Apply to team"),
                    ready && can_apply,
                ),
                (
                    cancel,
                    Kind::Apply(true),
                    tr!("Cancel application"),
                    ready && can_cancel,
                ),
            ] {
                button.project(commands, key, kind, caption, enabled);
            }
        }
        commands
            .entity(self.leader_feedback)
            .insert(Text::new(leader.feedback.clone()));

        self.close
            .project(commands, key, Kind::Close, tr!("Close"), true);
    }
}

/// The editor values for a listing, empty without one.
pub(super) fn listing_values(listing: Option<LeaderListing>) -> [String; LISTING_FIELDS] {
    let Some(listing) = listing else {
        return Default::default();
    };
    let (activity, id, area) = match listing.target {
        TeamTarget::Boss(id) => (1, id, String::new()),
        TeamTarget::Hunt { kind, area } => (2, kind, area.to_string()),
        TeamTarget::Quest(id) => (3, id, String::new()),
    };
    [
        listing.min_level.to_string(),
        listing.max_level.to_string(),
        listing.vocation_mask.to_string(),
        listing.slots.to_string(),
        listing.free_slots.to_string(),
        u8::from(listing.party_members_join).to_string(),
        activity.to_string(),
        id.to_string(),
        area,
    ]
}
