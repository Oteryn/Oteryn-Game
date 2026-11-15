use super::*;

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
                appearance_id: 0,
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
