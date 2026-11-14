use super::*;
use crate::{combat_input::PlayCommand, play::{Outbox, PlayView, play_channel}};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::hireling_management::wire;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (App, tokio::sync::mpsc::UnboundedReceiver<PlayCommand>, Outbox);
fn entry() -> wire::Entry {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    wire::Entry { hireling: id, name: "Alice".into(), sex: 2, look_type: 1107,
        colors: [1, 2, 3, 4], skills: 5, outfits: 1, position: Some((7, 9, -1)) }
}
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_hireling_management_initialize();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100,100),7,&[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {view,link});
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins,AssetPlugin::default()))
        .init_asset::<Font>().init_asset::<Image>().init_resource::<InputFocus>()
        .insert_non_send(owner).add_systems(Startup,crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app,commands,outbox))
}
fn panel(app: &App) -> std::result::Result<Panel, Box<dyn std::error::Error>> {
    Ok(app.world().non_send::<HostOwner>().play.as_ref().ok_or("play")?.link.hireling_management_panel())
}
fn control(app: &mut App, kind: Kind) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut().query::<(Entity,&Control)>().iter(app.world())
        .find_map(|(entity,control)| (control.kind==kind).then_some(entity)).ok_or("control missing".into())
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = control(app,kind)?;
    app.world_mut().trigger(Activate {entity});
    Ok(())
}
fn open_loaded(app: &mut App, commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>, outbox: &Outbox) -> Result {
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    let PlayCommand::Feature(request) = commands.try_recv()? else {return Err("list missing".into())};
    assert_eq!(wire::Request::decode(&request.payload),Some(wire::Request::List));
    outbox.bevy_hireling_management_reply(wire::Reply::State(vec![entry()]));
    app.update();
    click(app,Kind::Select(0))?;
    app.update();
    Ok(())
}
#[test]
fn hireling_management_window_shows_server_data_only() -> Result {
    let (mut app,mut commands,outbox)=fixture()?;
    open_loaded(&mut app,&mut commands,&outbox)?;
    let canonical=oteryn_client_ui::hireling_management::panel_text(&panel(&app)?);
    let texts=app.world_mut().query::<&Text>().iter(app.world()).map(|t|t.0.clone()).collect::<Vec<_>>();
    assert!(texts.contains(&canonical));
    assert!(canonical.contains("Alice") && canonical.contains("Placed at 7, 9, -1") && canonical.contains("Banker, Steward"));
    assert!(control(&mut app,Kind::Save(1107)).is_ok());
    assert!(control(&mut app,Kind::Save(1109)).is_ok());
    assert!(control(&mut app,Kind::Save(1113)).is_err());
    assert_eq!(panel(&app)?.entries, vec![entry()]);
    Ok(())
}
#[test]
fn hireling_management_actions_are_bound_and_confirmed() -> Result {
    let (mut app,mut commands,outbox)=fixture()?;
    open_loaded(&mut app,&mut commands,&outbox)?;
    click(&mut app,Kind::Save(1109))?;
    let PlayCommand::Feature(request)=commands.try_recv()? else {return Err("outfit missing".into())};
    assert_eq!(wire::Request::decode(&request.payload),Some(wire::Request::Outfit {hireling:entry().hireling,look_type:1109,colors:[1,2,3,4]}));
    assert_eq!(panel(&app)?.entries,vec![entry()]);
    click(&mut app,Kind::Save(1109))?;
    assert!(commands.try_recv().is_err());
    outbox.bevy_hireling_management_reply(wire::Reply::Committed);
    app.update();
    assert_eq!(panel(&app)?.entries[0].look_type,1109);
    click(&mut app,Kind::Return)?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app,Kind::Confirm)?;
    click(&mut app,Kind::Confirm)?;
    let PlayCommand::Feature(request)=commands.try_recv()? else {return Err("return missing".into())};
    assert_eq!(wire::Request::decode(&request.payload),Some(wire::Request::ReturnToLamp{hireling:entry().hireling}));
    assert!(commands.try_recv().is_err());
    assert!(panel(&app)?.entries[0].position.is_some());
    outbox.bevy_hireling_management_reply(wire::Reply::Committed);
    app.update();
    assert!(panel(&app)?.entries[0].position.is_none());
    Ok(())
}
#[test]
fn hireling_management_window_registered() -> Result {
    use bevy::input::keyboard::{Key,KeyboardInput};
    let (mut app,mut commands,outbox)=fixture()?;
    let toolbar=app.world_mut().query::<(Entity,&crate::bevy_shell::ui::Control)>().iter(app.world())
        .find_map(|(e,c)| (c.id()=="shell.hireling_management").then_some(e)).ok_or("toolbar missing")?;
    app.world_mut().trigger(Activate{entity:toolbar});
    assert!(panel(&app)?.open);
    let _=commands.try_recv()?;
    outbox.bevy_hireling_management_reply(wire::Reply::State(vec![]));
    app.update();
    click(&mut app,Kind::Close)?;
    app.update();
    assert!(!panel(&app)?.open);
    app.world_mut().write_message(KeyboardInput{key_code:KeyCode::F8,logical_key:Key::F8,
        state:bevy::input::ButtonState::Pressed,text:None,repeat:false,window:Entity::PLACEHOLDER});
    app.update();
    assert!(panel(&app)?.open);
    let _=commands.try_recv()?;
    app.world_mut().write_message(KeyboardInput{key_code:KeyCode::Escape,logical_key:Key::Escape,
        state:bevy::input::ButtonState::Pressed,text:None,repeat:false,window:Entity::PLACEHOLDER});
    app.update();
    assert!(!panel(&app)?.open);
    Ok(())
}
#[test]
fn hireling_management_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app,mut commands,outbox)=fixture()?;
    open_loaded(&mut app,&mut commands,&outbox)?;
    let stale=control(&mut app,Kind::Save(1109))?;
    let link=&app.world().non_send::<HostOwner>().play.as_ref().ok_or("play")?.link;
    let snapshot=link.hireling_management_snapshot();
    assert!(link.hireling_management_dispatch_bound(&snapshot,HirelingAction::Refresh));
    let _=commands.try_recv()?;
    let mut changed=entry();changed.name="Changed on server".into();
    outbox.bevy_hireling_management_reply(wire::Reply::State(vec![changed]));
    app.world_mut().trigger(Activate{entity:stale});
    assert!(commands.try_recv().is_err());
    app.update();
    let save=control(&mut app,Kind::Save(1109))?;
    let higher=app.world_mut().spawn((GameModal,GlobalZIndex(220),Node::default())).id();
    app.world_mut().trigger(Activate{entity:save});
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.world_mut().trigger(Activate{entity:save});
    assert!(commands.try_recv().is_err());
    app.update();
    assert_eq!(app.world_mut().query::<&Overlay>().iter(app.world()).count(),0);
    Ok(())
}
