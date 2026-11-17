use super::*;

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.prey_panel())
        .filter(|panel| panel.open);
    let Some(panel) = panel else {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        *state = State::default();
        return;
    };
    let generation = owner.generation();
    if state
        .seen
        .as_ref()
        .is_some_and(|(old, old_panel, _, _, _)| *old != generation || old_panel != &panel)
    {
        state.confirmation = None;
        state.wildcard = None;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let seen = (
        generation,
        panel.clone(),
        state.confirmation,
        state.wildcard.clone(),
        state.candidate,
    );
    if state.seen.as_ref() == Some(&seen) {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            GameModal,
            crate::bevy_shell::ui::window_layout::WindowLayer("prey"),
            GlobalZIndex(158),
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(8.),
                top: percent(12.),
                width: percent(84.),
                max_height: percent(80.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: px(24.).all(),
                row_gap: px(8.),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("prey", 0),
        ))
        .id();
    label(&mut commands, &skin, root, "Prey Creatures");
    button(
        &mut commands,
        &skin,
        root,
        generation,
        &panel,
        Kind::Close,
        "Close",
    );
    if let Some(confirmed) = &panel.confirmed {
        label(
            &mut commands,
            &skin,
            root,
            &format!("Wildcards: {}", confirmed.wildcard_balance),
        );
    }
    if let Some(reply) = &panel.confirmed {
        label(
            &mut commands,
            &skin,
            root,
            &format!("Prey Wildcards: {} / 52", reply.wildcard_balance),
        );
        label(
            &mut commands,
            &skin,
            root,
            &format!(
                "Creature list: {} · Expansion {}",
                reply.list_size,
                if reply.expansion_owned {
                    "owned"
                } else {
                    "not owned"
                }
            ),
        );
        label(
            &mut commands,
            &skin,
            root,
            "Controls requiring more Wildcards than your balance are disabled.",
        );
        button(
            &mut commands,
            &skin,
            root,
            generation,
            &panel,
            Kind::Store,
            "Get Wildcards in Store",
        );
    }
    if let Some(feedback) = panel.feedback {
        label(&mut commands, &skin, root, feedback);
    }
    if panel.pending.is_some() {
        label(&mut commands, &skin, root, "Waiting for the server…");
    } else if panel.selected {
        button(
            &mut commands,
            &skin,
            root,
            generation,
            &panel,
            Kind::Refresh,
            "Refresh",
        );
        if let Some(intent) = &state.wildcard {
            let prompt = match intent {
                Intent::SelectAny { slot, creature } => format!(
                    "Use 5 Prey wildcard cards for {} in slot {slot}?",
                    creature_label(creature)
                ),
                Intent::Select { slot } => {
                    let creature = state
                        .candidate
                        .and_then(|(_, index)| {
                            panel.slot(*slot)?.creatures.get(usize::from(index - 1))
                        })
                        .map_or_else(String::new, |key| creature_label(key));
                    format!(
                        "Choose {creature} in slot {slot}? Select another candidate or confirm Choose."
                    )
                }
                Intent::BonusReroll { slot } => {
                    format!("Reroll bonus in slot {slot} for 1 Wildcard?")
                }
                Intent::SetOption { slot, option } => format!(
                    "Enable {} in slot {slot}? Costs {} Wildcards per renewal.",
                    if *option == 1 {
                        "automatic bonus reroll"
                    } else {
                        "locked bonus"
                    },
                    if *option == 1 { 1 } else { 5 }
                ),
                _ => String::new(),
            };
            label(&mut commands, &skin, root, &prompt);
            let confirm = button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                match intent {
                    Intent::Select { slot } => {
                        Kind::Choose(*slot, state.candidate.map_or(0, |(_, i)| i))
                    }
                    _ => Kind::ConfirmAny(intent.clone()),
                },
                if matches!(intent, Intent::Select { .. }) {
                    "Choose"
                } else {
                    "Confirm wildcard selection"
                },
            );
            commands
                .entity(confirm)
                .insert(Name::new("prey.wildcard.confirm"));
            button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                Kind::Cancel,
                "Cancel",
            );
            if let Intent::Select { slot } = intent
                && let Some(data) = panel.slot(*slot)
            {
                for (index, creature) in data.creatures.iter().enumerate() {
                    if let Ok(index) = u8::try_from(index + 1) {
                        button(
                            &mut commands,
                            &skin,
                            root,
                            generation,
                            &panel,
                            Kind::Select(*slot, index),
                            &format!("Select {}", creature_label(creature)),
                        );
                    }
                }
            }
        } else if let Some(slot) = state.confirmation {
            label(
                &mut commands,
                &skin,
                root,
                &format!(
                    "Reroll the creature list in slot {slot}? The server may charge gold; the gold price is not provided. Rerolling replaces the current selection and bonus."
                ),
            );
            button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                Kind::Confirm,
                "Confirm reroll",
            );
            button(
                &mut commands,
                &skin,
                root,
                generation,
                &panel,
                Kind::Cancel,
                "Cancel",
            );
        } else if panel.confirmed.is_some() {
            for id in 1..=3 {
                let Some(slot) = panel.slot(id) else {
                    continue;
                };
                label(
                    &mut commands,
                    &skin,
                    root,
                    &format!("Slot {id} · {}", slot_title(slot.state)),
                );
                if slot.state != 0 {
                    let availability = label(
                        &mut commands,
                        &skin,
                        root,
                        &reroll_time(slot.free_reroll_at, now),
                    );
                    commands
                        .entity(availability)
                        .insert(FreeReroll(slot.free_reroll_at));
                }
                if slot.state == 0 && id >= 2 {
                    label(
                        &mut commands,
                        &skin,
                        root,
                        if id == 2 {
                            "Slot 2: Premium access or a permanent Store unlock."
                        } else {
                            "Slot 3: permanent Store unlock."
                        },
                    );
                    button(
                        &mut commands,
                        &skin,
                        root,
                        generation,
                        &panel,
                        Kind::Store,
                        &format!("Unlock slot {id} in Store"),
                    );
                    label(
                        &mut commands,
                        &skin,
                        root,
                        "After a purchase, reopen Prey and Refresh to check access.",
                    );
                }
                if slot.state == 2 {
                    let bonus = ["None", "Damage", "Defense", "Experience", "Loot"]
                        .get(usize::from(slot.bonus))
                        .copied()
                        .unwrap_or("Unknown");
                    label(
                        &mut commands,
                        &skin,
                        root,
                        &format!(
                            "{} · {bonus} +{}% · {}/10 · {:02}:{:02}",
                            creature_label(&slot.selected),
                            slot.percent,
                            slot.grade,
                            slot.hunting_seconds / 3600,
                            (slot.hunting_seconds / 60) % 60
                        ),
                    );
                }
                if slot.state == 1 {
                    for (index, creature) in slot.creatures.iter().enumerate() {
                        if let Ok(index) = u8::try_from(index + 1) {
                            button(
                                &mut commands,
                                &skin,
                                root,
                                generation,
                                &panel,
                                Kind::Select(id, index),
                                &format!("Select {}", creature_label(creature)),
                            );
                        }
                    }
                }
                if slot.state == 1 {
                    label(&mut commands, &skin, root, "Wildcard creature name");
                    let mut editor = EditableText::new("");
                    editor.max_characters = Some(256);
                    let field = commands
                        .spawn((
                            Wildcard(id),
                            TextInput,
                            editor,
                            Name::new(format!("prey.wildcard.creature.{id}")),
                            source::font(skin.font.clone(), false),
                            TextColor(source::TEXT_COLOR),
                            bevy::text::TextCursorStyle::default(),
                            bevy::input_focus::tab_navigation::TabIndex(0),
                            source::frame(skin.inset.clone(), [1.; 4]),
                            Node {
                                width: px(300.),
                                min_height: px(25.),
                                ..default()
                            },
                        ))
                        .id();
                    commands.entity(root).add_child(field);
                    let choose = button(
                        &mut commands,
                        &skin,
                        root,
                        generation,
                        &panel,
                        Kind::ChooseAny(id),
                        "Choose creature",
                    );
                    commands
                        .entity(choose)
                        .insert(Name::new(format!("prey.wildcard.choose.{id}")));
                }
                for kind in slot_actions(slot.state) {
                    let (kind, text) = match kind {
                        Kind::Reroll(_) => (Kind::Reroll(id), "List reroll".to_owned()),
                        Kind::Send(Intent::BonusReroll { .. }) => (
                            Kind::Send(Intent::BonusReroll { slot: id }),
                            "Bonus reroll · 1 Wildcard".to_owned(),
                        ),
                        Kind::Send(Intent::SetOption { option, .. }) => {
                            let text = format!(
                                "{} {}",
                                if slot.option == option { "[x]" } else { "[ ]" },
                                if option == 1 {
                                    "Auto-reroll · 1 Wildcard per renewal"
                                } else {
                                    "Lock bonus · 5 Wildcards per renewal"
                                }
                            );
                            (
                                Kind::Send(Intent::SetOption {
                                    slot: id,
                                    option: if slot.option == option { 0 } else { option },
                                }),
                                text,
                            )
                        }
                        _ => continue,
                    };
                    button(&mut commands, &skin, root, generation, &panel, kind, &text);
                }
            }
        } else {
            label(&mut commands, &skin, root, "Loading Prey…");
        }
    } else {
        label(
            &mut commands,
            &skin,
            root,
            "Prey is unavailable in this session.",
        );
    }
    state.root = Some(root);
    state.seen = Some(seen);
}
