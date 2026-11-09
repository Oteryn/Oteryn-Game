use super::*;
use oteryn_client_runtime::world_interaction::{Action, Target};

#[test]
fn targeting_click_sends_one_command_without_walking_or_attacking() -> Result<(), BatchError> {
    let mut view = view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .begin_targeting(std::num::NonZeroU64::MIN);
    view.primary_click(TileCoord::new(98, 200), &link)?;
    assert!(view.inputs.is_empty());
    assert!(
        matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::WorldContext(command)) if command.action == Action::UseOn(std::num::NonZeroU64::MIN) && command.target == Target::Position(ActorPosition {x:98,y:200,floor:0}))
    );
    // While pending, the real primary-click route must consume the click too.
    view.primary_click(TileCoord::new(99, 200), &link)?;
    assert!(view.inputs.is_empty());
    assert!(commands.try_recv().is_err());
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .pending = false;
    view.primary_click(TileCoord::new(99, 200), &link)?;
    assert!(matches!(view.inputs.front(), Some(Input::TargetWalk(_))));
    Ok(())
}

#[test]
fn escape_cancels_only_active_targeting_and_shutdown_clears_it() {
    let (link, mut commands, outbox) = play_channel();
    let escape = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::ESCAPE,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::NONE,
        repeat: false,
    };
    assert!(!link.use_with_cancel_input(std::slice::from_ref(&escape)));
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .begin_targeting(std::num::NonZeroU64::MIN);
    assert!(link.use_with_cancel_input(std::slice::from_ref(&escape)));
    assert!(!link.use_with_cancel_input(std::slice::from_ref(&escape)));
    assert!(commands.try_recv().is_err());
    super::super::mailbox::lock(&link.mailbox)
        .world_context
        .begin_targeting(std::num::NonZeroU64::MIN);
    outbox.inventory_end();
    assert!(!link.use_with_cancel_input(std::slice::from_ref(&escape)));
}

#[test]
fn pending_and_self_targets_preserve_targeting_and_send_nothing() -> Result<(), BatchError> {
    let view = view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    let source = std::num::NonZeroU64::MIN;
    {
        let mut mailbox = super::super::mailbox::lock(&link.mailbox);
        mailbox.world_context.begin_targeting(source);
        mailbox.world_context.pending = true;
    }
    assert!(view.use_with_click(TileCoord::new(98, 200), &link));
    assert!(commands.try_recv().is_err());
    assert_eq!(
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        Some(source)
    );
    assert!(
        link.world_context_feedback()
            .is_some_and(|text| text.contains("waiting"))
    );
    Ok(())
}

fn carried(handle: u64) -> Result<oteryn_session::ItemEntry, Box<dyn std::error::Error>> {
    Ok(oteryn_session::ItemEntry {
        handle: handle.try_into()?,
        item_definition_ref: 1.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    })
}
fn item_map_view() -> Result<PlayView, Box<dyn std::error::Error>> {
    use oteryn_session::world_map::*;
    let mut view = view()?;
    view.set_live_map(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 0,
            origin: ActorPosition {
                x: 100,
                y: 200,
                floor: 0,
            },
            first_visible_floor: 0,
        },
        tiles: [100, 101]
            .into_iter()
            .map(|x| MapTile {
                position: ActorPosition {
                    x,
                    y: 200,
                    floor: 0,
                },
                items: vec![MapItem {
                    definition: MapDefinition::Item(NonZeroU32::MIN),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::Handle(std::num::NonZeroU64::MIN),
                    appearance_id: 0,
                }],
                more: false,
                ground_speed: 0,
            })
            .collect(),
    })?;
    Ok(view)
}
#[test]
fn rune_uses_position_on_item_tiles_including_own_tile_and_tools_keep_items()
-> Result<(), Box<dyn std::error::Error>> {
    for (rune, x) in [(true, 101), (true, 100), (false, 101)] {
        let mut view = item_map_view()?;
        let (link, mut commands, outbox) = play_channel();
        outbox.targeting_initialize(true);
        let item = carried(2)?;
        outbox.inventory_initialize(
            Some(oteryn_session::CharacterInventory {
                main_backpack: Some(item),
                ..Default::default()
            }),
            false,
        );
        assert!(link.inventory_select(item.handle));
        assert!(if rune {
            link.inventory_use_rune()
        } else {
            link.inventory_use_with()
        });
        view.primary_click(TileCoord::new(x, 200), &link)?;
        let crate::combat_input::PlayCommand::WorldContext(command) = commands.try_recv()? else {
            return Err("unexpected command".into());
        };
        assert_eq!(
            command.target,
            if rune {
                Target::Position(ActorPosition {
                    x,
                    y: 200,
                    floor: 0,
                })
            } else {
                Target::Item(std::num::NonZeroU64::MIN)
            }
        );
        assert_eq!(
            super::super::mailbox::lock(&link.mailbox)
                .world_context
                .targeting,
            None
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn self_target_keeps_mode_and_inventory_destination_consumes_once()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = item_map_view()?;
    let (link, mut commands, outbox) = play_channel();
    outbox.targeting_initialize(true);
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(carried(1)?),
            entries: vec![carried(2)?],
            equipment: vec![],
        }),
        false,
    );
    assert!(link.inventory_select(std::num::NonZeroU64::MIN));
    assert!(link.inventory_use_with());
    assert!(!link.inventory_use_with());
    view.primary_click(TileCoord::new(101, 200), &link)?;
    assert!(commands.try_recv().is_err());
    assert_eq!(
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        Some(std::num::NonZeroU64::MIN)
    );
    assert!(link.inventory_select(2.try_into()?));
    assert!(link.inventory_use_with());
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::WorldContext(command) if command.action==Action::UseOn(std::num::NonZeroU64::MIN) && command.target==Target::Item(2.try_into()?))
    );
    assert!(!link.inventory_use_with());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn published_source_removal_cancels_targeting() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    outbox.targeting_initialize(true);
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(carried(1)?),
            ..Default::default()
        }),
        false,
    );
    assert!(link.inventory_select(std::num::NonZeroU64::MIN));
    assert!(link.inventory_use_with());
    let session = Fake {
        serves: Arc::new(AtomicUsize::new(0)),
        fail_step: false,
        fail_serve_after: usize::MAX,
        queued: vec![],
        drained: Arc::new(AtomicUsize::new(0)),
        combats: Arc::new(Mutex::new(vec![])),
        logouts: Arc::default(),
    };
    outbox.targeting_refresh(&session);
    assert_eq!(
        super::super::mailbox::lock(&link.mailbox)
            .world_context
            .targeting,
        None
    );
    assert!(
        link.world_context_feedback()
            .is_some_and(|text| text.contains("disappeared"))
    );
    Ok(())
}

#[test]
fn native_inventory_targeting_keeps_other_panels_alive() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(carried(1)?),
            ..Default::default()
        }),
        false,
    );
    outbox.targeting_initialize(true);
    assert!(link.inventory_select(1.try_into()?));
    assert!(PlayLink::inventory_use_shared(
        &link.mailbox,
        &link.commands,
        false
    ));
    let mailbox = super::super::mailbox::lock(&link.mailbox);
    assert_eq!(mailbox.world_context.targeting, Some(1.try_into()?));
    assert!(!mailbox.inventory_shutdown);
    assert!(!mailbox.depot_shutdown);
    assert!(!mailbox.market_shutdown);
    assert!(!mailbox.quest_shutdown);
    assert!(!mailbox.wheel_shutdown);
    Ok(())
}

#[path = "play_additional.rs"]
mod additional;
