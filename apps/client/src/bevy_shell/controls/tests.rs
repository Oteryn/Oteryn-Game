use super::*;
use bevy::{ecs::system::RunSystemOnce, input_focus::InputFocus, prelude::*};
use oteryn_input_actions::{ButtonState, KeyCode as OwnerKey, Modifier, Modifiers};
type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Exercise the same editor/modal admission as the raw-winit adapter.
pub(in crate::bevy_shell) fn shortcut(
    app: &mut App,
    code: u16,
    modifiers: Modifiers,
) -> TestResult {
    let blocked = app
        .world_mut()
        .run_system_once(
            |focus: Res<InputFocus>,
             editors: Query<(), With<bevy::text::EditableText>>,
             guard: crate::bevy_shell::game_ui::modal::ModalGuard| {
                focus.get().is_some_and(|entity| editors.contains(entity))
                    || !guard.allows_focus(None)
            },
        )
        .map_err(|e| format!("{e:?}"))?;
    let event = NormalizedInputEvent::Key {
        code: OwnerKey::new(code)?,
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], blocked);
    Ok(())
}

#[test]
fn routed_owner_chords_open_windows_and_chat_focus_admits_none() -> TestResult {
    for (code, modifiers, window) in [
        (
            65,
            Modifiers::one(Modifier::Control).with(Modifier::Alt),
            "coins",
        ),
        (10, Modifiers::one(Modifier::Control), "guild"),
        (
            5,
            Modifiers::one(Modifier::Control).with(Modifier::Alt),
            "bank",
        ),
        (8, Modifiers::one(Modifier::Control), "exiva"),
        (
            22,
            Modifiers::one(Modifier::Control).with(Modifier::Shift),
            "store",
        ),
        (
            19,
            Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            "party",
        ),
        (
            23,
            Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            "potions",
        ),
        (
            24,
            Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            "mechanics",
        ),
        (
            16,
            Modifiers::one(Modifier::Control).with(Modifier::Shift),
            "notices",
        ),
        (
            17,
            Modifiers::one(Modifier::Control).with(Modifier::Alt),
            "training",
        ),
        (
            22,
            Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            "cast",
        ),
        (17, Modifiers::one(Modifier::Control), "world"),
    ] {
        for chat in [false, true] {
            let (link, _commands, outbox) = crate::play::play_channel();
            outbox.bevy_cast_initialize();
            link.use_shared_frontend();
            if chat {
                link.open_chat();
                if !link.preference_chat_open() {
                    link.chat_console_mode();
                }
                assert!(link.preference_chat_open());
            }
            let mut owner = HostOwner::new(None)?;
            // Global binds Ctrl+E to Close Current Channel by default; expose the owner's
            // optional chord without bypassing preference routing or user bindings.
            owner.settings.preferences.custom_hotkeys.remove("CTRL+E");
            owner
                .settings
                .preferences
                .rebind("0147-exiva-settings-open", "CTRL+E")?;
            let mut view =
                crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
            view.use_shared_frontend();
            owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
            owner.phase = crate::bevy_shell::owner::Phase::Playing;
            owner.route_input(
                &[NormalizedInputEvent::Key {
                    code: OwnerKey::new(code)?,
                    state: ButtonState::Pressed,
                    modifiers,
                    repeat: false,
                }],
                false,
            );
            let link = &owner.play.as_ref().ok_or("play")?.link;
            let open = match window {
                "coins" => link.market_panel().coins.open,
                "guild" => link.guild_panel().open,
                "bank" => link.guild_bank_panel().open,
                "exiva" => link.exiva_panel().open,
                "store" => link.store_panel().open,
                "party" => link.party_effects_panel().open,
                "potions" => link.status_details_panel().active() == Some(crate::play::feature_0280_party_effects::protocol::bound::StatusDetailsTab::Potions),
                "mechanics" => link.status_details_panel().active() == Some(crate::play::feature_0280_party_effects::protocol::bound::StatusDetailsTab::Mechanics),
                "notices" => link.achievement_notices_binding().panel.open,
                "training" => link.training_panel().open,
                "cast" => link.cast_control_panel().open,
                "world" => link.world_event_panel().open,
                _ => false,
            };
            assert_eq!(open, !chat, "{window}: chat={chat}");
        }
    }
    Ok(())
}

#[test]
fn routed_cast_chord_rejects_super_and_blocked_input() -> TestResult {
    for (super_key, blocked) in [(true, false), (false, true)] {
        let (link, mut commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_cast_initialize();
        let mut owner = HostOwner::new(None)?;
        let mut view =
            crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        let modifiers = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        owner.route_input(
            &[NormalizedInputEvent::Key {
                code: OwnerKey::new(22)?,
                state: ButtonState::Pressed,
                modifiers: if super_key {
                    modifiers.with(Modifier::Super)
                } else {
                    modifiers
                },
                repeat: false,
            }],
            blocked,
        );
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(!link.cast_control_panel().open);
        assert!(!link.store_panel().open);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
