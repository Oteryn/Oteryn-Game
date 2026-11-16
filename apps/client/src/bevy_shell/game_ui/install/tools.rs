use super::*;
pub(super) fn project_tools(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    skin: Res<Skins>,
    guard: ModalGuard,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut state: ResMut<ToolUi>,
) {
    if let Some(request) = owner.take_container_tool_request()
        && request.generation == owner.generation()
        && owner.play.is_some()
    {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.request = Some(request);
        state.seen = None;
    }
    if owner.play.is_none()
        || state
            .request
            .is_some_and(|request| request.generation != owner.generation())
        || (!owner.modal_active()
            && state.root.is_some_and(|root| guard.allows(root))
            && keys
                .as_ref()
                .is_some_and(|keys| keys.just_pressed(KeyCode::Escape)))
    {
        state.request = None;
    }
    let Some(request) = state.request else {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.seen = None;
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let snapshot = ToolSnapshot::read(&play.link, request.tool);
    if state.seen.as_ref() == Some(&snapshot) {
        return;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        state.request = None;
        return;
    };
    state.revision = revision;
    let root = *state.root.get_or_insert_with(|| {
        commands
            .spawn((
                super::super::GameModal,
                GlobalZIndex(155),
                Name::new(request.tool.title()),
                bevy::ui_widgets::ScrollArea,
                RememberScroll("container_tools", request.tool as u32),
                source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(20.),
                    top: percent(10.),
                    width: percent(60.),
                    max_height: percent(80.),
                    padding: px(20.).all(),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4.),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .id()
    });
    commands.entity(root).despawn_children();
    source::dialog_caption(&mut commands, root, request.tool.title(), skin.font.clone());
    let text = commands
        .spawn(source::text(
            snapshot.text(),
            source::font(skin.font.clone(), false),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(root).add_child(text);
    let mut button = |label: &str, action, enabled| {
        tool_button(&mut commands, &skin, root, &state, label, action, enabled)
    };
    for tool in [
        ContainerTool::BrowseField,
        ContainerTool::Depot,
        ContainerTool::Search,
        ContainerTool::Sort,
    ] {
        button(tool.title(), ToolAction::Change(tool), true);
    }
    let ready = snapshot.ready();
    match &snapshot {
        ToolSnapshot::Browse(panel) => {
            button("Browse selected tile", ToolAction::Browse, ready);
            for view in &panel.views.views {
                if let Some(page) = view.browse {
                    for (label, index, enabled) in [
                        (
                            "Previous page",
                            page.index.saturating_sub(30),
                            page.index > 0,
                        ),
                        (
                            "Next page",
                            page.index.saturating_add(30),
                            page.index.saturating_add(30) < page.total,
                        ),
                    ] {
                        button(
                            label,
                            ToolAction::Seek {
                                view: view.view_id,
                                handle: view.container_handle,
                                index,
                            },
                            ready && enabled,
                        );
                    }
                }
            }
        }
        ToolSnapshot::Depot(panel) => {
            let (box_number, page) = panel
                .reply
                .as_ref()
                .map_or((1, 0), |reply| (reply.box_number, reply.page));
            for (label, box_number, page, enabled) in [
                ("Refresh box", box_number, page, true),
                (
                    "Previous box",
                    box_number.saturating_sub(1),
                    0,
                    box_number > 1,
                ),
                ("Next box", box_number.saturating_add(1), 0, box_number < 17),
                (
                    "Previous page",
                    box_number,
                    page.saturating_sub(1),
                    page > 0,
                ),
                (
                    "Next page",
                    box_number,
                    page.saturating_add(1),
                    page < 468
                        && panel
                            .reply
                            .as_ref()
                            .is_some_and(|reply| !reply.entries.is_empty()),
                ),
            ] {
                button(
                    label,
                    ToolAction::DepotBrowse { box_number, page },
                    ready && enabled,
                );
            }
            if let Some(reply) = &panel.reply {
                for (row, entry) in reply.entries.iter().enumerate() {
                    button(
                        &format!("Withdraw item {} ×{}", entry.definition, entry.count),
                        ToolAction::Withdraw(row),
                        ready && reply.town.is_some(),
                    );
                }
                for item in oteryn_client_ui::depot::deposit_items(panel) {
                    button(
                        &format!("Deposit item {} ×{}", item.item_definition_ref, item.count),
                        ToolAction::Deposit(item.handle),
                        ready && reply.town.is_some(),
                    );
                }
            }
        }
        ToolSnapshot::Search(panel) => {
            use oteryn_client_runtime::cui3::depot_search::{Outcome, Request};
            button(
                "List depot holdings",
                ToolAction::Search(Request::Open { after: 0 }),
                ready,
            );
            if let Some(reply) = &panel.reply {
                if reply.outcome == Outcome::More
                    && let Some(last) = reply.results.last()
                {
                    button(
                        "Next page",
                        ToolAction::Search(Request::Open {
                            after: last.client_id,
                        }),
                        ready,
                    );
                }
                for row in &reply.results {
                    for all in [false, true] {
                        button(
                            &format!(
                                "Retrieve {} of item {}",
                                if all { "all" } else { "one" },
                                row.client_id
                            ),
                            ToolAction::Search(Request::Retrieve {
                                client_id: row.client_id,
                                all,
                            }),
                            ready,
                        );
                    }
                    for tier in &row.tiers {
                        button(
                            &format!("Item {} tier {} details", row.client_id, tier.tier),
                            ToolAction::Search(Request::Detail {
                                client_id: row.client_id,
                                tier: tier.tier,
                            }),
                            ready,
                        );
                    }
                }
            }
        }
        ToolSnapshot::Sort(_, containers, mode) => {
            if mode.selected {
                button(
                    "Use Manual Sort Mode",
                    ToolAction::ManualSort {
                        checked: mode.active(),
                    },
                    ready,
                );
            }
            if let Some(views) = &containers.views {
                for view in &views.views {
                    if view.browse.is_some()
                        || request
                            .view
                            .is_some_and(|source| source != (view.view_id, view.container_handle))
                    {
                        continue;
                    }
                    use oteryn_client_runtime::container_sort::SortKey;
                    for (key, label) in [
                        (SortKey::Name, "name"),
                        (SortKey::Weight, "weight"),
                        (SortKey::Expiry, "expiry"),
                        (SortKey::StackSize, "stack size"),
                    ] {
                        for descending in [false, true] {
                            button(
                                &format!(
                                    "Sort container {} by {label} {}",
                                    view.view_id,
                                    if descending {
                                        "descending"
                                    } else {
                                        "ascending"
                                    }
                                ),
                                ToolAction::Sort {
                                    view: view.view_id,
                                    handle: view.container_handle,
                                    key,
                                    descending,
                                },
                                ready,
                            );
                        }
                    }
                }
            }
        }
    }
    button("Close", ToolAction::Close, true);
    state.seen = Some(snapshot);
}
