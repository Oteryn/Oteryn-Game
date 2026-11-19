use super::*;
use bevy::{asset::AssetPlugin, input_focus::InputFocus, prelude::*};

#[test]
fn a_started_login_updater_closes_the_existing_bevy_owner() -> Result<(), Box<dyn std::error::Error>>
{
    let mut owner = HostOwner::new(None)?;
    owner.phase = Phase::Entering;
    owner.login.show();
    owner.finish_admission(
        Err(crate::GameplayEntryError::ClientUpdateRequired {
            manual_download_url: "https://oteryn.com/download".into(),
            updater_started: true,
        }),
        None,
    );
    assert_eq!(owner.phase, Phase::Closed);
    assert!(!owner.login.visible);
    assert!(owner.entry.is_none());
    Ok(())
}

#[test]
fn hidden_reentry_refusal_projects_the_login_alert() -> Result<(), Box<dyn std::error::Error>> {
    use super::super::{login, ui};

    let mut owner = HostOwner::new(None)?;
    owner.phase = Phase::Entering;
    assert!(!owner.login.visible);
    owner.finish_admission(Err(crate::GameplayEntryError::ChannelFull), None);
    assert_eq!(owner.phase, Phase::Offline);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, (ui::setup, login::setup).chain())
    .add_systems(Update, login::project);
    app.update();
    let world = app.world_mut();
    let alert = world
        .query_filtered::<&Node, With<login::LoginAlert>>()
        .single(world)?;
    assert_eq!(alert.display, Display::Flex);
    let message = world
        .query_filtered::<&Text, With<login::AlertText>>()
        .single(world)?;
    assert!(message.0.contains("This channel is full."));
    Ok(())
}
