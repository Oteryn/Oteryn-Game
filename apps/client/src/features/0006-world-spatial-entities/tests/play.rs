use super::*;

#[test]
fn snapshot_builds_a_view_centred_on_the_own_position_with_overlay_markers()
-> Result<(), BatchError> {
    let view = view()?;
    // Only the marker on the own floor and with a decodable placement is drawn.
    assert_eq!(view.marker_count(), 1);
    assert_eq!(view.markers.len(), 2);
    let scene = view.scene();
    assert_eq!(scene.sprites().len(), 2);
    let [x, y] = scene.view().tile_to_screen(TileCoord::new(100, 200));
    assert_eq!(
        scene.view().screen_to_tile(x + 1.0, y + 1.0),
        Some(TileCoord::new(100, 200))
    );
    assert_eq!(scene.view().origin(), TileCoord::new(93, 195));
    Ok(())
}

#[test]
fn generated_dispatch_keeps_spatial_event_order_and_ignores_undrawn_domains() -> Result<(), String>
{
    let position = ActorPosition {
        x: 103,
        y: 201,
        floor: 0,
    };
    let entities = SessionEvent::WorldSpatialEntities(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::WorldSpatialEntitiesDelta {
            content_generation: [0; 32],
            actor_position: position,
            enter: Vec::new(),
            update: Vec::new(),
            leave: Vec::new(),
        },
    });
    let ignored = SessionEvent::ActorVitals(AppliedDelta {
        server_sequence: 3,
        base_revision: 1,
        new_revision: 2,
        value: oteryn_session::ActorVitals::default(),
    });
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![spatial_event(1, 50), entities, ignored]);
    let Some(PlayEvent::Pushed(pushed)) = link.poll() else {
        return Err("missing pushed event".to_owned());
    };
    assert_eq!(pushed.own, Some(position));
    let mut view = view().map_err(|e| e.to_string())?;
    view.apply_pushed(&pushed).map_err(|e| e.to_string())?;
    assert_eq!(view.own(), TileCoord::new(103, 201));
    outbox.push(vec![spatial_event(4, 51)]);
    let Some(PlayEvent::Pushed(pushed)) = link.poll() else {
        return Err("missing pushed event".to_owned());
    };
    assert_eq!(pushed.own.map(|p| p.x), Some(51));
    Ok(())
}

#[test]
fn live_names_reach_scene_and_disappear_with_visibility() -> Result<(), BatchError> {
    let mut view = view()?;
    let entity = oteryn_session::WorldSpatialEntity {
        display_name: Some(
            oteryn_session::EntityDisplayName::new("Rat").map_err(|_| BatchError::InvalidView)?,
        ),
        kind: oteryn_session::EntityKind::Creature,
        entity: oteryn_session::EntityRef {
            identity: [8; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let mut appearance = oteryn_session::VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity,
        look: None,
    };
    view.set_presentations(vec![appearance.clone()])?;
    assert_eq!(view.scene().name_count(), 1);
    let unmarked = view.scene().sprites().len();
    for stacks in [1, 5, 15] {
        appearance.forge_stacks = Some(stacks);
        view.set_presentations(vec![appearance.clone()])?;
        assert!(view.scene().sprites().len() > unmarked);
    }
    appearance.forge_stacks = None;
    view.set_presentations(vec![appearance])?;
    assert_eq!(view.scene().sprites().len(), unmarked);
    view.set_presentations(Vec::new())?;
    assert_eq!(view.scene().name_count(), 0);
    Ok(())
}

#[test]
fn own_identity_draws_once_between_step_and_appearance_publish() -> Result<(), BatchError> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    view.set_own_identity([1; 16]);
    let own = oteryn_session::VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity: oteryn_session::WorldSpatialEntity {
            display_name: None,
            kind: oteryn_session::EntityKind::Player,
            entity: oteryn_session::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
    };
    let mut other = own.clone();
    other.entity.entity.identity = [8; 16];
    other.entity.kind = oteryn_session::EntityKind::Creature;
    other.look = Some(oteryn_session::Look::EmptyOutfit);
    view.set_presentations(vec![own.clone(), other.clone()])?;
    assert_eq!(view.scene().sprites().len(), 1);
    view.apply(&moved_to(101, 200, 0))?;
    assert_eq!(
        view.scene().sprites().len(),
        1,
        "stale own position must not leave a ghost"
    );
    assert_eq!(
        view.scene().sprites().instances()[0].position,
        view.scene().view().tile_to_screen(TileCoord::new(101, 200))
    );
    let mut updated = own;
    updated.entity.position.x = 101;
    view.set_presentations(vec![updated, other])?;
    assert_eq!(view.scene().sprites().len(), 1);
    Ok(())
}

#[test]
fn live_presentations_keep_target_clock_and_light_through_rebuilds() -> Result<(), BatchError> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let entity = WorldSpatialEntity {
        display_name: Some(
            oteryn_session::EntityDisplayName::new("Rat").map_err(|_| BatchError::InvalidView)?,
        ),
        kind: oteryn_session::EntityKind::Creature,
        entity: EntityRef {
            identity: [8; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    view = view.with_entities(vec![entity])?;
    view.click(TileCoord::new(101, 200))?;
    let selected = view.scene().target();
    assert!(selected.is_some());
    let appearance = oteryn_session::VisibleAppearance {
        forge_stacks: None,
        light: None,
        entity,
        look: None,
    };
    view.apply_pushed(&Pushed {
        appearances: Some(vec![appearance.clone()]),
        world_clock: Some(oteryn_session::world_clock::WorldClock {
            tibian_minute: 1439,
            light_level: 128,
            light_color: 180,
        }),
        ..Pushed::default()
    })?;
    let lit = view.scene().sprites().lighting.clone();
    let mut unlit = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    unlit.set_presentations(vec![appearance])?;
    assert_eq!(lit.ambient, [128, 0, 0]);
    assert_ne!(lit, unlit.scene().sprites().lighting);
    assert_eq!(view.scene().name_count(), 1);
    assert!(view.hud_status().contains("23:59"));
    assert_eq!(view.scene().target(), selected);
    view.apply_pushed(&Pushed::default())?;
    assert_eq!(view.scene().sprites().lighting, lit);
    assert_eq!(view.scene().target(), selected);
    Ok(())
}

#[test]
fn echo_portal_click_walks_and_reward_remains_in_native_container()
-> Result<(), Box<dyn std::error::Error>> {
    use super::feature_local_world_view::live_snapshot;
    use oteryn_session::world_map::{decode_world_map_snapshot, encode_world_map_snapshot};
    let mut map = live_snapshot(101);
    // The real PG producer emits this appearance with a session handle.
    // Compact definition references remain generation-specific.
    map.tiles[0].items[0].appearance_id = 54133;
    map.tiles[0].items[0].origin = oteryn_session::world_map::MapOrigin::Handle(1.try_into()?);
    let bytes = encode_world_map_snapshot(&map).map_err(|e| format!("{e:?}"))?;
    let decoded = decode_world_map_snapshot(&bytes).map_err(|e| format!("{e:?}"))?;
    let mut play = view()?;
    play.set_live_map(&decoded)?;
    let portal_quads = play.scene().sprites().len();
    play.click(TileCoord::new(101, 200))?;
    assert_eq!(
        play.inputs.back(),
        Some(&Input::TargetWalk(ActorPosition {
            x: 101,
            y: 200,
            floor: 0
        })),
        "a volatile Echo portal must not swallow the click needed to step into it"
    );
    assert_eq!(
        play.scene().sprites().len(),
        portal_quads,
        "walking does not erase the server map"
    );
    // Ordinary handled items on the same tile remain selectable.
    let mut ordinary = decoded.clone();
    ordinary.tiles[0].items[0].appearance_id = 42;
    play.inputs.clear();
    play.set_live_map(&ordinary)?;
    play.click(TileCoord::new(101, 200))?;
    assert!(play.inputs.is_empty());
    assert!(
        play.scene()
            .target()
            .and_then(|target| target.item)
            .is_some()
    );
    play.set_live_map(&decoded)?;
    // The server removes the portal after activation; no local encounter or payout is invented.
    map.tiles[0].items.clear();
    play.set_live_map(&map)?;
    assert!(play.scene().target().is_none());
    let (link, _commands, outbox) = play_channel();
    use oteryn_client_runtime::container_tree::{ContainerView, ContainerViews};
    let rewards = ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: 90.try_into()?,
            parent_handle: None,
            capacity: 20,
            entries: vec![
                oteryn_session::ItemEntry {
                    handle: 91.try_into()?,
                    // Compact generation reference, not a donor item ID.
                    item_definition_ref: 42.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                },
                oteryn_session::ItemEntry {
                    handle: 92.try_into()?,
                    item_definition_ref: 43.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                },
            ],
            browse: None,
        }],
    };
    outbox.container_initialize(Some(ContainerViews::default()));
    outbox.container_events(&[SessionEvent::ContainerViews(AppliedDelta {
        server_sequence: 3,
        base_revision: 1,
        new_revision: 2,
        value: {
            use oteryn_session::container_tree::{decode_container_views, encode_container_views};
            let bytes = encode_container_views(&rewards).map_err(|e| format!("{e:?}"))?;
            decode_container_views(&bytes).map_err(|e| format!("{e:?}"))?
        },
    })]);
    let panel = link.container_panel();
    assert_eq!(
        panel.current().ok_or("missing reward container")?.entries,
        rewards.views[0].entries
    );
    assert!(oteryn_client_ui::container_tree::view::panel_text(&panel).contains("2 / 20"));
    outbox.container_end();
    assert!(link.container_panel().current().is_none());
    Ok(())
}

include!("play_skinning_decoded_stationary_corpse_updates_native_look_target_1.rs");
