use super::*;
use bevy::{ecs::system::RunSystemOnce, input_focus::FocusCause};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn permits(app: &mut App, entity: Entity) -> Result<bool, Box<dyn std::error::Error>> {
    app.world_mut()
        .run_system_once(move |guard: ModalGuard| guard.allows(entity))
        .map_err(|error| format!("modal fixture: {error:?}").into())
}
#[test]
fn hidden_ancestors_are_ignored_and_explicit_global_layers_escape_parents() -> TestResult {
    let mut app = App::new();
    let outside = app.world_mut().spawn_empty().id();
    assert!(permits(&mut app, outside)?);
    let hidden = app
        .world_mut()
        .spawn(Node {
            display: Display::None,
            ..default()
        })
        .id();
    let blocked = app
        .world_mut()
        .spawn((
            GameModal,
            Node::default(),
            GlobalZIndex(500),
            ChildOf(hidden),
        ))
        .id();
    assert!(permits(&mut app, outside)?);
    app.world_mut()
        .get_mut::<Node>(hidden)
        .ok_or("hidden")?
        .display = Display::Flex;
    app.world_mut()
        .entity_mut(hidden)
        .insert(Visibility::Hidden);
    assert!(permits(&mut app, outside)?);
    app.world_mut()
        .entity_mut(hidden)
        .insert(Visibility::Inherited);
    assert!(!permits(&mut app, outside)?);
    app.world_mut()
        .get_mut::<Node>(blocked)
        .ok_or("blocked")?
        .display = Display::None;
    let parent = app
        .world_mut()
        .spawn((Node::default(), GlobalZIndex(300)))
        .id();
    let low = app
        .world_mut()
        .spawn((
            GameModal,
            Node::default(),
            GlobalZIndex(20),
            ChildOf(parent),
        ))
        .id();
    let high = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(180)))
        .id();
    assert!(permits(&mut app, high)?);
    assert!(!permits(&mut app, low)?);
    Ok(())
}
#[test]
fn nested_modal_inherits_its_global_layer_and_its_own_scrim_can_dismiss() -> TestResult {
    let mut app = App::new();
    let overlay = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(140)))
        .id();
    let panel = app
        .world_mut()
        .spawn((GameModal, Node::default(), ChildOf(overlay)))
        .id();
    let button = app
        .world_mut()
        .spawn((Node::default(), ChildOf(panel)))
        .id();
    let low = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(80)))
        .id();
    assert!(permits(&mut app, button)?);
    assert!(permits(&mut app, overlay)?);
    assert!(!permits(&mut app, low)?);
    let high = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(180)))
        .id();
    assert!(permits(&mut app, high)?);
    assert!(!permits(&mut app, button)?);
    assert!(!permits(&mut app, overlay)?);
    Ok(())
}
#[test]
fn local_sibling_z_order_is_respected_without_computed_layout() -> TestResult {
    let mut app = App::new();
    let root = app.world_mut().spawn(Node::default()).id();
    let high = app
        .world_mut()
        .spawn((GameModal, Node::default(), ZIndex(5), ChildOf(root)))
        .id();
    let low = app
        .world_mut()
        .spawn((GameModal, Node::default(), ZIndex(1), ChildOf(root)))
        .id();
    assert!(permits(&mut app, high)?);
    assert!(!permits(&mut app, low)?);
    Ok(())
}
#[test]
fn sanitizer_clears_outside_focus_preserves_modal_focus_and_never_restores_it() -> TestResult {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install(&mut app);
    let outside = app.world_mut().spawn_empty().id();
    let modal = app.world_mut().spawn((GameModal, Node::default())).id();
    let inside = app
        .world_mut()
        .spawn((Node::default(), ChildOf(modal)))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(outside, FocusCause::Navigated);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(inside, FocusCause::Navigated);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(inside));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(outside, FocusCause::Navigated);
    app.update();
    app.world_mut()
        .get_mut::<Node>(modal)
        .ok_or("modal")?
        .display = Display::None;
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    let none_allowed = app
        .world_mut()
        .run_system_once(|guard: ModalGuard| guard.allows_focus(None))
        .map_err(|error| format!("focus fixture: {error:?}"))?;
    assert!(none_allowed);
    Ok(())
}

#[test]
fn closed_or_despawned_top_freezes_the_frame_instead_of_releasing_lower_handlers() -> TestResult {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install(&mut app);
    let low = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(80)))
        .id();
    let high = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(180)))
        .id();
    app.update();
    assert!(permits(&mut app, high)?);
    app.world_mut().get_mut::<Node>(high).ok_or("high")?.display = Display::None;
    assert!(!permits(&mut app, low)?);
    assert!(!permits(&mut app, high)?);
    app.update();
    assert!(permits(&mut app, low)?);
    // A new higher modal observed mid-frame also tightens the cache until frame end.
    let newer = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(200)))
        .id();
    assert!(permits(&mut app, newer)?);
    app.world_mut().entity_mut(newer).despawn();
    assert!(!permits(&mut app, low)?);
    app.update();
    assert!(permits(&mut app, low)?);
    Ok(())
}
#[test]
fn hidden_widgets_do_not_act_or_keep_focus_even_without_any_modal() -> TestResult {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install(&mut app);
    let root = app
        .world_mut()
        .spawn(Node {
            display: Display::None,
            ..default()
        })
        .id();
    let widget = app.world_mut().spawn((Node::default(), ChildOf(root))).id();
    assert!(!permits(&mut app, widget)?);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(widget, FocusCause::Navigated);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    Ok(())
}

#[test]
fn raising_the_same_top_also_tightens_its_retained_frame_layer() -> TestResult {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install(&mut app);
    let high = app
        .world_mut()
        .spawn((GameModal, Node::default(), ZIndex(2)))
        .id();
    app.update();
    app.world_mut().entity_mut(high).insert(ZIndex(4));
    let middle = app
        .world_mut()
        .spawn((GameModal, Node::default(), ZIndex(3)))
        .id();
    assert!(permits(&mut app, high)?);
    app.world_mut().get_mut::<Node>(high).ok_or("high")?.display = Display::None;
    assert!(!permits(&mut app, middle)?);
    app.update();
    assert!(permits(&mut app, middle)?);
    Ok(())
}

#[test]
fn equal_global_nested_panels_win_independently_of_archetype_iteration_order() -> TestResult {
    let mut app = App::new();
    let unrelated = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(80)))
        .id();
    let scrim = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(80)))
        .id();
    let panel = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(80), ChildOf(scrim)))
        .id();
    let outside_panel = app
        .world_mut()
        .spawn((Node::default(), ChildOf(scrim)))
        .id();
    let inside = app
        .world_mut()
        .spawn((Node::default(), ChildOf(panel)))
        .id();
    // Move the scrim to an archetype visited after the nested panel. UI ordering
    // must still follow the hierarchy, not ECS query iteration order.
    app.world_mut()
        .entity_mut(scrim)
        .insert(Name::new("context scrim"));
    assert!(permits(&mut app, inside)?);
    assert!(permits(&mut app, scrim)?);
    assert!(!permits(&mut app, outside_panel)?);
    assert!(!permits(&mut app, unrelated)?);
    Ok(())
}

#[test]
fn upstream_computed_paint_order_wins_over_headless_root_indices() -> TestResult {
    use bevy::ui::{ComputedStackIndex, UiStack};
    let mut app = App::new();
    let first = app
        .world_mut()
        .spawn((
            GameModal,
            Node::default(),
            GlobalZIndex(80),
            ComputedStackIndex(1),
        ))
        .id();
    let second = app
        .world_mut()
        .spawn((
            GameModal,
            Node::default(),
            GlobalZIndex(80),
            ComputedStackIndex(0),
        ))
        .id();
    // UiStack is Bevy's back-to-front output, not an entity allocation order.
    app.insert_resource(UiStack {
        uinodes: vec![second, first],
        ..default()
    });
    assert!(permits(&mut app, first)?);
    assert!(!permits(&mut app, second)?);
    // A reordered upstream stack changes the permitted root without changing entity ids.
    app.world_mut()
        .get_mut::<ComputedStackIndex>(first)
        .ok_or("first")?
        .0 = 0;
    app.world_mut()
        .get_mut::<ComputedStackIndex>(second)
        .ok_or("second")?
        .0 = 1;
    app.world_mut().resource_mut::<UiStack>().uinodes = vec![first, second];
    assert!(permits(&mut app, second)?);
    assert!(!permits(&mut app, first)?);
    Ok(())
}
