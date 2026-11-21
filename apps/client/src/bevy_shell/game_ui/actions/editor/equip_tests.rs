use super::tests::{TestResult, captioned, control, set};
use super::*;

#[test]
fn equip_editor_does_not_assume_tier_zero_on_tiered_worlds() -> TestResult {
    check_equip_tier(true, "", "3")
}

#[test]
fn equip_editor_rejects_explicit_tiers_on_legacy_worlds() -> TestResult {
    check_equip_tier(false, "3", "")
}

#[test]
fn equip_editor_keeps_out_of_range_tiers_out_of_multiaction_drafts() -> TestResult {
    check_equip_tier(true, "11", "3")
}

fn check_equip_tier(tiered: bool, bad_tier: &str, good_tier: &str) -> TestResult {
    let (mut app, _) = super::super::tests::fixture()?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            entries: vec![oteryn_session::ItemEntry {
                appearance_id: 0,
                handle: 9.try_into()?,
                item_definition_ref: 256.try_into()?,
                count: 1.try_into()?,
                sub_type: 0,
            }],
            ..Default::default()
        }),
        Some(Default::default()),
        true,
        true,
    );
    outbox.bevy_hotkey_equip_items(vec![256]);
    outbox.bevy_equip_capabilities(tiered);
    let generation = {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.play.as_mut().ok_or("play")?.link = link;
        owner.generation()
    };
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    let object = captioned(&mut app, "Object 256")?;
    let equip = control(&mut app, |edit| matches!(edit, Edit::Equip))?;
    let multi = control(&mut app, |edit| matches!(edit, Edit::Multi))?;
    let use_item = control(&mut app, |edit| matches!(edit, Edit::Use))?;
    app.world_mut().trigger(Activate { entity: object });
    set(&mut app, 5, bad_tier);
    app.world_mut().trigger(Activate { entity: equip });
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("Equip"),
        "tier {bad_tier:?}, tiered {tiered}"
    );
    app.world_mut().trigger(Activate { entity: multi });
    app.world_mut().trigger(Activate { entity: use_item });
    app.world_mut().trigger(Activate { entity: equip });
    assert_eq!(
        app.world()
            .resource::<EditorState>()
            .multi
            .as_ref()
            .map(Vec::len),
        Some(1)
    );
    set(&mut app, 5, good_tier);
    app.world_mut().trigger(Activate { entity: equip });
    app.world_mut().trigger(Activate { entity: multi });
    assert_eq!(
        app.world().resource::<EditorState>().message,
        "Multiaction saved"
    );
    {
        let owner = &mut *app.world_mut().non_send_mut::<HostOwner>();
        owner
            .settings
            .sync_bar_actions(&owner.play.as_ref().ok_or("play")?.link);
    }
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .bar_actions
            .0
            .get(&0),
        Some(&BarAction::Multi {
            actions: vec![
                BarAction::Use {
                    definition: 256u32.try_into()?,
                    sub_type: Some(0),
                    mode: ItemUseMode::Use,
                },
                BarAction::Equip {
                    definition: 256u32.try_into()?,
                    upgrade_tier: if tiered { 3 } else { 0 },
                },
            ],
        })
    );
    app.world_mut().trigger(Activate { entity: equip });
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cast_spell_slot(std::num::NonZeroU32::MIN);
    let mut payload = vec![0, 0, 1, 0];
    if tiered {
        payload.push(3);
    }
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::ServerMechanic(cmd)
            if cmd.command_type == 348 && cmd.payload == payload)
    );
    Ok(())
}

#[test]
fn saved_equip_does_not_send_a_legacy_request_after_reconnect() -> TestResult {
    check_saved_equip_on_legacy_world(LegacyEquipCase::Single)
}

#[test]
fn saved_multiaction_equip_does_not_send_a_legacy_request_after_reconnect() -> TestResult {
    check_saved_equip_on_legacy_world(LegacyEquipCase::Multi)
}

#[test]
fn reopened_multiaction_with_unsupported_equip_tier_cannot_be_saved() -> TestResult {
    check_saved_equip_on_legacy_world(LegacyEquipCase::ReopenMulti)
}

#[test]
fn saved_multiaction_dispatches_ready_equip_after_unsupported_tier_on_reconnect() -> TestResult {
    check_saved_equip_on_legacy_world(LegacyEquipCase::MultiWithFallback)
}

enum LegacyEquipCase {
    Single,
    Multi,
    MultiWithFallback,
    ReopenMulti,
}

fn check_saved_equip_on_legacy_world(case: LegacyEquipCase) -> TestResult {
    let (mut app, _) = super::super::tests::fixture()?;
    let (tiered, _, tiered_outbox) = crate::play::play_channel();
    tiered_outbox.bevy_equip_capabilities(true);
    tiered_outbox.bevy_hotkey_equip_items(vec![256, 257]);
    let equip = BarAction::Equip {
        definition: 256u32.try_into()?,
        upgrade_tier: 3,
    };
    let action = match case {
        LegacyEquipCase::Single => equip,
        LegacyEquipCase::Multi | LegacyEquipCase::ReopenMulti => BarAction::Multi {
            actions: vec![equip],
        },
        LegacyEquipCase::MultiWithFallback => BarAction::Multi {
            actions: vec![
                equip,
                BarAction::Equip {
                    definition: 257u32.try_into()?,
                    upgrade_tier: 0,
                },
            ],
        },
    };
    assert!(tiered.assign_bar_action(0, action.clone()));
    let (legacy, mut commands, legacy_outbox) = crate::play::play_channel();
    legacy_outbox.bevy_equip_capabilities(false);
    legacy_outbox.bevy_hotkey_equip_items(vec![256, 257]);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.sync_bar_actions(&tiered);
        assert_eq!(
            owner.settings.preferences.bar_actions.0.get(&0),
            Some(&action)
        );
        owner.settings.sync_bar_actions(&legacy);
        owner.play.as_mut().ok_or("play")?.link = legacy;
    }
    let slot = super::super::tests::slot(&mut app, 0)?;
    if matches!(case, LegacyEquipCase::ReopenMulti) {
        let generation = app.world().non_send::<HostOwner>().generation();
        app.world_mut()
            .resource_mut::<EditorState>()
            .open(0, generation);
        app.update();
        let toggle = control(&mut app, |edit| matches!(edit, Edit::Multi))?;
        app.world_mut().trigger(Activate { entity: toggle });
        app.world_mut().trigger(Activate { entity: toggle });
        let editor = app.world().resource::<EditorState>();
        assert_ne!(editor.message, "Multiaction saved");
        assert!(editor.multi.is_some(), "refused save must keep the draft");
    } else {
        app.world_mut().trigger(Activate { entity: slot });
        if matches!(case, LegacyEquipCase::MultiWithFallback) {
            assert!(
                matches!(commands.try_recv()?, crate::combat_input::PlayCommand::ServerMechanic(cmd)
                    if cmd.command_type == 348 && cmd.payload == [0, 0, 1, 1])
            );
        }
        assert!(
            commands.try_recv().is_err(),
            "unexpected legacy Equip sent from {action:?}"
        );
    }
    Ok(())
}
