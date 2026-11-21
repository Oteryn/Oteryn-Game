use super::*;
use bevy::input::{
    ButtonState,
    keyboard::{Key as LogicalKey, KeyboardInput},
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn one_escape_closes_the_top_notice_without_closing_the_underlying_journal() -> Result {
    let (host, _commands, _outbox) = super::tests::host()?;
    let key = Key {
        generation: host.generation(),
        tab: Tab::Quests,
    };
    let link = &host.play.as_ref().ok_or("play")?.link;
    link.quest_set_open(true);
    link.notices_set_open(true);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(host)
        .insert_resource(State {
            key: Some(key),
            ..default()
        })
        .add_message::<KeyboardInput>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    super::super::feature_dialogs::install_notice_input_for_test(&mut app);
    app.add_systems(
        Update,
        keyboard.after(super::super::feature_dialogs::NoticeInputForTest),
    );
    app.world_mut().spawn((
        Overlay,
        GameModal,
        GlobalZIndex(155),
        Node {
            width: percent(100.),
            height: percent(100.),
            ..default()
        },
    ));
    app.update();
    let escape = || KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: LogicalKey::Escape,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(escape());
    app.update();
    {
        let host = app.world().get_non_send::<HostOwner>().ok_or("host")?;
        let link = &host.play.as_ref().ok_or("play")?.link;
        assert!(
            !link.notices_panel().open,
            "top notice must close on Escape"
        );
        assert!(
            link.quest_panel().open,
            "the same Escape must not close the underlying journal"
        );
    }
    app.world_mut().write_message(escape());
    app.update();
    let host = app.world().get_non_send::<HostOwner>().ok_or("host")?;
    assert!(
        !host.play.as_ref().ok_or("play")?.link.quest_panel().open,
        "a new Escape may close the now-top journal"
    );
    Ok(())
}
