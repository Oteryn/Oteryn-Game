use super::*;

#[test]
fn multi_action_uses_first_ready_spell_through_play_queue_and_persists()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{ActorCooldown, ActorCooldownKind, ActorCooldowns, spell_catalog::*};
    let (link, mut commands, outbox) = play_channel();
    let mut view = view()?;
    let entries = (1..=3)
        .map(|id| SpellCatalogEntry {
            cast_index: id,
            name: format!("Spell{id}"),
            words: format!("words{id}"),
            level: 1,
            mana: 1,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![id],
            rune_item: None,
        })
        .collect::<Vec<_>>();
    outbox.spell_catalog_initialize(
        true,
        Some(encode_catalog(&entries).map_err(|e| format!("{e:?}"))?),
    )?;
    let action = serde_json::from_str(
        r#"{"kind":"Multi","actions":[{"kind":"Spell","name":"Spell1","words":"words1"},{"kind":"Spell","name":"Spell2","words":"words2"},{"kind":"Spell","name":"Spell3","words":"words3"}]}"#,
    )?;
    assert!(link.assign_bar_action(0, action));
    for (blocked, expected) in [
        (vec![1, 2], Some(3)),
        (vec![1], Some(2)),
        (vec![], Some(1)),
        (vec![1, 2, 3], None),
    ] {
        outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorCooldowns {
                server_now_ms: 100,
                rtt_ms: None,
                entries: blocked
                    .into_iter()
                    .map(|id| ActorCooldown {
                        kind: if id == 1 {
                            ActorCooldownKind::Spell
                        } else {
                            ActorCooldownKind::Group
                        },
                        id,
                        expires_at_ms: 60100,
                    })
                    .collect(),
            },
        })]);
        if let Some(index) = expected {
            assert!(link.spell_slot_labels()[0].contains(&format!("Next {index}/3: Spell{index}")));
        }
        view.cast(std::num::NonZeroU32::MIN);
        view.tick(&link).map_err(|_| "tick")?;
        if let Some(index) = expected {
            assert!(
                matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == index)
            );
        }
        assert!(commands.try_recv().is_err());
    }
    let path = std::env::temp_dir().join(format!("multi-action-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.sync_bar_actions(&link);
    assert_eq!(
        SettingsState::new(Some(path.clone()))
            .preferences
            .bar_actions,
        settings.preferences.bar_actions
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn each_bar_accepts_fifty_buttons_through_play_queue() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, _) = play_channel();
    let mut view = view()?;
    for slot in [49, 99, 149] {
        {
            let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
            mailbox.chat.selected = true;
            mailbox.chat.pending = false;
        }
        assert!(link.assign_bar_action(
            slot,
            crate::play::BarAction::Text {
                text: "fifty".into(),
                send_automatically: true
            }
        ));
        view.cast(std::num::NonZeroU32::new(slot as u32 + 1).ok_or("slot")?);
        view.tick(&link).map_err(|_| "tick")?;
        assert!(commands.try_recv().is_ok());
    }
    assert!(!link.assign_bar_action(
        150,
        crate::play::BarAction::Text {
            text: "overflow".into(),
            send_automatically: true
        }
    ));
    Ok(())
}

#[test]
fn cooldown_bar_consumes_real_server_events_independently_of_action_slots()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{ActorCooldown, ActorCooldownKind, ActorCooldowns};
    let (link, _, outbox) = play_channel();
    let mut view = view()?;
    outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: ActorCooldowns {
            server_now_ms: 100,
            rtt_ms: None,
            entries: vec![
                ActorCooldown {
                    kind: ActorCooldownKind::Group,
                    id: 2,
                    expires_at_ms: 1100,
                },
                ActorCooldown {
                    kind: ActorCooldownKind::Spell,
                    id: 71,
                    expires_at_ms: 600,
                },
            ],
        },
    })]);
    view.tick(&link).map_err(|_| "tick")?;
    crate::play::mailbox::lock(&link.mailbox)
        .local_preferences
        .action_bar
        .visible = false;
    let icons = link.cooldown_bar_icons();
    assert_eq!(icons.len(), 2);
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.local_preferences.action_bar.cooldown_bar_enabled = false;
    }
    assert!(link.cooldown_bar_icons().is_empty());
    crate::play::mailbox::lock(&link.mailbox)
        .local_preferences
        .action_bar
        .cooldown_bar_enabled = true;
    assert!(
        icons
            .iter()
            .any(|icon| !icon.group && icon.id == 71 && icon.remaining_ms > 0)
    );
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.spell_bar.cooldown_received_at =
            std::time::Instant::now().checked_sub(std::time::Duration::from_secs(2));
    }
    let icons = link.cooldown_bar_icons();
    assert_eq!(icons.len(), 1);
    assert!(icons[0].group && icons[0].remaining_ms == 0 && icons[0].overlay_per_mille == 0);
    outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: ActorCooldowns {
            server_now_ms: 1200,
            rtt_ms: None,
            entries: vec![],
        },
    })]);
    view.tick(&link).map_err(|_| "tick")?;
    assert_eq!(
        link.cooldown_bar_icons().len(),
        1,
        "group icons survive empty snapshots"
    );
    Ok(())
}

#[test]
fn cursor_position_item_uses_real_play_queue_without_second_click()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, ItemUseMode};
    use oteryn_client_runtime::world_interaction::{Selection, Target};
    use oteryn_session::{CharacterInventory, ItemEntry};
    use std::num::{NonZeroU32, NonZeroU64};
    let (link, mut commands, outbox) = play_channel();
    let mut view = view()?;
    outbox.targeting_initialize(true);
    outbox.inventory_initialize(
        Some(CharacterInventory {
            entries: vec![ItemEntry {
                handle: NonZeroU64::MIN,
                item_definition_ref: NonZeroU32::new(256).ok_or("item")?,
                count: NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        }),
        true,
    );
    assert!(link.assign_bar_action(
        0,
        BarAction::Use {
            definition: NonZeroU32::new(256).ok_or("item")?,
            mode: ItemUseMode::CursorPosition
        }
    ));
    view.cast(NonZeroU32::MIN);
    view.tick(&link).map_err(|_| "tick")?;
    assert!(
        commands.try_recv().is_err(),
        "outside the window must not start crosshair targeting"
    );
    assert!(
        crate::play::mailbox::lock(&link.mailbox)
            .world_context
            .targeting
            .is_none()
    );
    let position = oteryn_session::ActorPosition {
        x: 100,
        y: 201,
        floor: 7,
    };
    crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = Some((
        Target::Position(position),
        Selection {
            position,
            binding: None,
        },
    ));
    view.cast(NonZeroU32::MIN);
    view.tick(&link).map_err(|_| "tick")?;
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::WorldContext(command) if command.target == Target::Position(position))
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn spell_aim_modes_and_diagonal_cursor_use_real_play_queue()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, SpellAim};
    use oteryn_client_runtime::world_interaction::{Selection, Target};
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
        spell_catalog::*,
    };
    let (link, mut commands, outbox) = play_channel();
    let mut view = view()?;
    let entry = SpellCatalogEntry {
        cast_index: 71,
        name: "Wave".into(),
        words: "wave".into(),
        level: 1,
        mana: 1,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![],
        rune_item: None,
    };
    outbox.spell_catalog_initialize(
        true,
        Some(encode_catalog(&[entry]).map_err(|e| format!("{e:?}"))?),
    )?;
    let actor = WorldSpatialEntity {
        entity: EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        display_name: None,
        kind: EntityKind::Player,
        position: ActorPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    outbox.hotkey_own_actor(Some(actor));
    for (aim, cursor, expected) in [
        (
            SpellAim::Facing,
            None,
            crate::combat_input::CombatCommand::CastFacing {
                spell: std::num::NonZeroU32::new(71).ok_or("spell")?,
                direction: None,
            },
        ),
        (
            SpellAim::Target,
            None,
            crate::combat_input::CombatCommand::Cast(std::num::NonZeroU32::new(71).ok_or("spell")?),
        ),
        (
            SpellAim::Cursor,
            Some((99, 99)),
            crate::combat_input::CombatCommand::CastFacing {
                spell: std::num::NonZeroU32::new(71).ok_or("spell")?,
                direction: Some(StepDirection::West),
            },
        ),
        (
            SpellAim::Cursor,
            Some((101, 101)),
            crate::combat_input::CombatCommand::CastFacing {
                spell: std::num::NonZeroU32::new(71).ok_or("spell")?,
                direction: Some(StepDirection::East),
            },
        ),
        (
            SpellAim::Cursor,
            Some((100, 99)),
            crate::combat_input::CombatCommand::CastFacing {
                spell: std::num::NonZeroU32::new(71).ok_or("spell")?,
                direction: Some(StepDirection::North),
            },
        ),
    ] {
        assert!(link.assign_bar_action(
            0,
            BarAction::Spell {
                name: "Wave".into(),
                words: "wave".into(),
                aim
            }
        ));
        assert!(link.bar_gesture(crate::play::BarGesture::Aim { slot: 0, aim }));
        crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = cursor.map(|(x, y)| {
            let position = ActorPosition { x, y, floor: 7 };
            (
                Target::Position(position),
                Selection {
                    position,
                    binding: None,
                },
            )
        });
        view.cast(std::num::NonZeroU32::MIN);
        view.tick(&link).map_err(|_| "tick")?;
        assert_eq!(
            commands.try_recv()?,
            crate::combat_input::PlayCommand::Combat(expected)
        );
    }
    Ok(())
}

#[test]
fn native_battle_keys_cast_catalogue_bindings_and_obey_death_gate()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{Modifier, Modifiers};
    use oteryn_session::spell_catalog::*;
    let (link, mut commands, outbox) = play_channel();
    assert!(link.bind_action_key(0, "SUPER+ALT+F12"));
    let keys = [
        (0x31, Modifiers::NONE),
        (0x7b, Modifiers::one(Modifier::Super).with(Modifier::Alt)),
    ];
    for cast_index in [71, 97] {
        let entry = SpellCatalogEntry {
            cast_index,
            name: "Light".into(),
            words: "utevo lux".into(),
            level: 1,
            mana: 1,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![],
            rune_item: None,
        };
        outbox.spell_catalog_initialize(
            true,
            Some(encode_catalog(&[entry]).map_err(|e| format!("{e:?}"))?),
        )?;
        if cast_index == 71 {
            assert!(link.assign_spell_slot(0, Some(cast_index)));
        }
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        assert!(mailbox.bar_actions.saved.0.is_empty());
        for (key, modifiers) in keys {
            assert!(
                crate::play::feature_local_spell_action_bar::view::native_key(
                    &mut mailbox,
                    &link.commands,
                    key,
                    modifiers
                )
            );
            assert!(
                matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == cast_index)
            );
            assert!(commands.try_recv().is_err());
        }
    }
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    mailbox.apply_death_vitals(oteryn_session::ActorVitals {
        health: 0,
        max_health: 100,
        ..Default::default()
    });
    for (key, modifiers) in keys {
        assert!(
            crate::play::feature_local_spell_action_bar::view::native_key(
                &mut mailbox,
                &link.commands,
                key,
                modifiers
            )
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn native_battle_key_uses_hovered_entity_generation_without_crosshair()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, ItemUseMode};
    use oteryn_client_runtime::world_interaction::{Selection, Target};
    use oteryn_input_actions::{Modifier, Modifiers};
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    let definition = std::num::NonZeroU32::new(256).ok_or("item")?;
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            entries: vec![oteryn_session::ItemEntry {
                handle: std::num::NonZeroU64::MIN,
                item_definition_ref: definition,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        }),
        true,
    );
    assert!(link.assign_bar_action(
        0,
        BarAction::Use {
            definition,
            mode: ItemUseMode::CursorPosition
        }
    ));
    assert!(link.bind_action_key(0, "SUPER+ALT+F12"));
    let target = oteryn_session::EntityRef {
        identity: [8; 16],
        generation: 9,
    };
    let position = oteryn_session::ActorPosition {
        x: 100,
        y: 101,
        floor: 7,
    };
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    mailbox.bar_actions.cursor = Some((
        Target::Creature(target),
        Selection {
            position,
            binding: None,
        },
    ));
    assert!(
        crate::play::feature_local_spell_action_bar::view::native_key(
            &mut mailbox,
            &link.commands,
            0x7b,
            Modifiers::one(Modifier::Super).with(Modifier::Alt)
        )
    );
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::WorldContext(command) if command.target == Target::Creature(target))
    );
    assert!(mailbox.world_context.targeting.is_none());
    mailbox.world_context.pending = false;
    assert!(
        crate::play::feature_local_spell_action_bar::view::native_key(
            &mut mailbox,
            &link.commands,
            0x31,
            Modifiers::NONE
        )
    );
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::WorldContext(command) if command.target == Target::Creature(target))
    );
    Ok(())
}

#[test]
fn multiaction_skips_rune_group_cooldown_then_uses_live_item_handle()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, ItemUseMode, SpellAim};
    use oteryn_client_runtime::world_interaction::{Selection, Target};
    use oteryn_session::{ActorCooldown, ActorCooldownKind, ActorCooldowns, spell_catalog::*};
    use std::num::{NonZeroU32, NonZeroU64};
    let (link, mut commands, outbox) = play_channel();
    let mut view = view()?;
    let definition = NonZeroU32::new(256).ok_or("definition")?;
    let entries = [
        SpellCatalogEntry {
            cast_index: 71,
            name: "Rune".into(),
            words: String::new(),
            level: 1,
            mana: 1,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![2],
            rune_item: Some(256),
        },
        SpellCatalogEntry {
            cast_index: 72,
            name: "Spell".into(),
            words: "spell".into(),
            level: 1,
            mana: 1,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![1],
            rune_item: None,
        },
    ];
    outbox.spell_catalog_initialize(
        true,
        Some(encode_catalog(&entries).map_err(|e| format!("{e:?}"))?),
    )?;
    outbox.targeting_initialize(true);
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            entries: vec![oteryn_session::ItemEntry {
                handle: NonZeroU64::MIN,
                item_definition_ref: definition,
                count: NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        }),
        true,
    );
    let position = oteryn_session::ActorPosition {
        x: 100,
        y: 101,
        floor: 7,
    };
    crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = Some((
        Target::Position(position),
        Selection {
            position,
            binding: None,
        },
    ));
    assert!(link.assign_bar_action(
        0,
        BarAction::Multi {
            actions: vec![
                BarAction::Use {
                    definition,
                    mode: ItemUseMode::CursorPosition
                },
                BarAction::Spell {
                    name: "Spell".into(),
                    words: "spell".into(),
                    aim: SpellAim::Target
                }
            ]
        }
    ));
    for blocked in [true, false] {
        outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorCooldowns {
                server_now_ms: 100,
                rtt_ms: None,
                entries: if blocked {
                    vec![ActorCooldown {
                        kind: ActorCooldownKind::Group,
                        id: 2,
                        expires_at_ms: 60100,
                    }]
                } else {
                    vec![]
                },
            },
        })]);
        view.cast(NonZeroU32::MIN);
        view.tick(&link).map_err(|_| "tick")?;
        if blocked {
            assert!(
                matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(index)) if index.get()==72)
            );
        } else {
            assert!(
                matches!(commands.try_recv()?,crate::combat_input::PlayCommand::WorldContext(command) if command.action==oteryn_client_runtime::world_interaction::Action::UseOn(NonZeroU64::MIN))
            );
        }
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
