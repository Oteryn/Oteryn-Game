use super::*;
use bevy::{
    input::InputPlugin,
    input_focus::InputDispatchPlugin,
    text::{RemSize, TextPlugin},
    ui_widgets::TextInputPlugin,
    window::WindowPlugin,
};
use std::sync::{Arc, Mutex};
type Fixture = (
    App,
    Entity,
    Entity,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn editor_app() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (host, commands, outbox) = fixture()?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        InputPlugin,
        InputDispatchPlugin,
        WindowPlugin::default(),
        TextPlugin,
        TextInputPlugin,
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .init_resource::<UiScale>()
    .init_resource::<RemSize>()
    .init_resource::<bevy::picking::events::PointerState>()
    .add_message::<bevy::picking::events::PointerRelease>()
    .insert_non_send(host)
    .add_systems(Startup, (crate::bevy_shell::ui::setup, test_font).chain())
    .add_systems(PostUpdate, bevy::ui::widget::update_editable_text_styles);
    super::super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let select = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, a)| matches!(a.kind, Kind::Select(_)))
        .map(|(e, _)| e)
        .ok_or("offer")?;
    app.world_mut().trigger(Activate { entity: select });
    app.update();
    let amount = field(&mut app, false)?;
    let buy = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, a)| matches!(a.kind, Kind::Trade))
        .map(|(e, _)| e)
        .ok_or("buy")?;
    app.world_mut()
        .get_mut::<EditableText>(amount)
        .ok_or("amount")?
        .queue_edit(bevy::text::TextEdit::SelectAll);
    app.update();
    Ok((app, amount, buy, commands, outbox))
}
fn test_font(mut skins: ResMut<Skins>, mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/assets/FiraMono-subset.ttf"
        ))
        .to_vec(),
    );
    skins.font = fonts.add(font);
}
fn field(app: &mut App, search: bool) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .find(|(_, f)| matches!(f, Field::Search) == search)
        .map(|(e, _)| e)
        .ok_or_else(|| "editor".into())
}
#[test]
fn buy_waits_for_same_frame_destructive_edit_and_sends_completed_amount() -> Result {
    let (mut app, amount, buy, mut commands, _) = editor_app()?;
    app.world_mut()
        .get_mut::<EditableText>(amount)
        .ok_or("amount")?
        .queue_edit(bevy::text::TextEdit::Insert("7".into()));
    app.world_mut().trigger(Activate { entity: buy });
    assert!(
        commands.try_recv().is_err(),
        "queued edit must not trade the old amount"
    );
    app.update();
    let PlayCommand::NpcTrade(intent) = commands.try_recv()? else {
        return Err("trade".into());
    };
    assert_eq!(intent.quantity, 7);
    assert_eq!(intent.catalogue_revision, 7);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn buy_waits_across_pending_clipboard_read_and_sends_pasted_amount_once() -> Result {
    let (mut app, amount, buy, mut commands, _) = editor_app()?;
    let clipboard = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(amount)
        .ok_or("amount")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &clipboard,
    )));
    app.world_mut().trigger(Activate { entity: buy });
    assert!(
        commands.try_recv().is_err(),
        "pending paste must not trade the old amount"
    );
    app.update();
    assert!(commands.try_recv().is_err());
    *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok("9".into()));
    app.update();
    let PlayCommand::NpcTrade(intent) = commands.try_recv()? else {
        return Err("trade".into());
    };
    assert_eq!(intent.quantity, 9);
    assert_eq!(intent.expected_unit_price, 12);
    app.update();
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn deferred_buy_cancels_on_quote_mode_generation_modal_or_search_changes() -> Result {
    for scope in 0..5 {
        let (mut app, amount, buy, mut commands, outbox) = editor_app()?;
        let editor = if scope == 4 {
            field(&mut app, true)?
        } else {
            amount
        };
        let clipboard = Arc::new(Mutex::new(None));
        app.world_mut()
            .get_mut::<EditableText>(editor)
            .ok_or("editor")?
            .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
            &clipboard,
        )));
        app.world_mut().trigger(Activate { entity: buy });
        assert!(commands.try_recv().is_err());
        app.update();
        match scope {
            0 => publish(&outbox, 8, 15),
            1 => {
                let side = app
                    .world_mut()
                    .query::<(Entity, &Action)>()
                    .iter(app.world())
                    .find(|(_, a)| matches!(a.kind, Kind::Side(NpcTradeSide::Sell)))
                    .map(|(e, _)| e)
                    .ok_or("sell tab")?;
                app.world_mut().trigger(Activate { entity: side });
            }
            2 => app.world_mut().non_send_mut::<HostOwner>().close(),
            3 => {
                app.world_mut().spawn((
                    super::super::super::GameModal,
                    GlobalZIndex(180),
                    Node::default(),
                ));
            }
            _ => {}
        }
        *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok(if scope == 4 {
            "no matching goods".into()
        } else {
            "5".into()
        }));
        app.update();
        app.update();
        assert!(
            commands.try_recv().is_err(),
            "pending trade must not retarget scope {scope}"
        );
    }
    Ok(())
}

#[test]
fn buy_while_server_request_is_pending_cannot_arm_a_late_clipboard_replay() -> Result {
    let (mut app, amount, buy, mut commands, outbox) = editor_app()?;
    app.world_mut().trigger(Activate { entity: buy });
    assert!(matches!(commands.try_recv()?, PlayCommand::NpcTrade(_)));
    let clipboard = Arc::new(Mutex::new(None));
    app.world_mut()
        .get_mut::<EditableText>(amount)
        .ok_or("amount")?
        .pending_paste = Some(bevy::clipboard::ClipboardRead::Pending(Arc::clone(
        &clipboard,
    )));
    app.world_mut().trigger(Activate { entity: buy });
    // A real refreshed shop window completes the first request without changing its quote.
    publish(&outbox, 7, 12);
    *clipboard.lock().map_err(|_| "clipboard lock")? = Some(Ok("5".into()));
    app.update();
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "a click refused while pending must not replay after the reply"
    );
    Ok(())
}
