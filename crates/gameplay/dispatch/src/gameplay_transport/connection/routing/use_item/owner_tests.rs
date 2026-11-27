use super::*;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FirstEntryOutcome, FreshAdmissionAttempt, SessionContinuity,
};
use crate::gameplay_transport::item_view::{
    InventoryItems, ItemTargetObservation, UseItemTarget, ViewItem,
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, MapUseTarget, UseDisposition, WorldObjectTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, EntityKind, EntityRef, WorldSpatialEntity,
};
use std::{cell::Cell, num::NonZeroU32};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owner {
    Stone,
    Food,
    Corpse,
    Container,
    Cube,
    Rune,
    Callback,
    Door,
    Lever,
    Tool,
    HouseItem,
}

struct Authority {
    expected: Owner,
    seen: Cell<Option<Owner>>,
}

impl Authority {
    fn use_with(&self, owner: Owner) -> UseOutcome {
        assert_eq!(self.expected, owner);
        assert_eq!(self.seen.replace(Some(owner)), None);
        UseOutcome {
            disposition: UseDisposition::Committed,
            ..UseOutcome::rejected()
        }
    }
}

impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn item_use_kind(&self, _: NonZeroU32) -> Option<ItemUseKind> {
        Some(match self.expected {
            Owner::Stone => ItemUseKind::Stone,
            Owner::Food => ItemUseKind::Food,
            Owner::Container => ItemUseKind::Container,
            Owner::Cube => ItemUseKind::Cube,
            Owner::Rune => ItemUseKind::Rune,
            _ => ItemUseKind::Other,
        })
    }

    async fn use_stone(
        &self, _: ExactActorRef, _: UseCommand, _: Option<ItemKey>, _: NonZeroU32,
    ) -> UseOutcome {
        self.use_with(Owner::Stone)
    }

    async fn use_known_food(
        &self, _: ExactActorRef, _: UseCommand, _: Option<ItemKey>, _: NonZeroU32,
    ) -> Option<UseOutcome> {
        Some(self.use_with(Owner::Food))
    }

    async fn use_drome_cube(
        &self, _: ExactActorRef, _: UseCommand, _: UseIntent, _: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        Ok(Some(self.use_with(Owner::Cube)))
    }

    async fn use_rune(
        &self, _: ExactActorRef, _: UseCommand, _: UseIntent, key: Option<ItemKey>,
        _: bool, _: bool,
    ) -> Option<UseOutcome> {
        assert!(matches!(key, Some(ItemKey::Instance(_))));
        Some(self.use_with(Owner::Rune))
    }

    async fn use_event_item(
        &self, _: ExactActorRef, _: UseCommand, _: UseIntent, key: Option<ItemKey>, _: bool,
    ) -> Result<Option<UseOutcome>, ()> {
        let owner = if matches!(key, Some(ItemKey::MapAdded(_))) {
            Owner::Callback
        } else {
            Owner::HouseItem
        };
        Ok(Some(self.use_with(owner)))
    }

    async fn use_object(
        &self, _: ExactActorRef, _: UseCommand, target: WorldObjectTarget,
    ) -> UseOutcome {
        let owner = if target.placement == b"door" { Owner::Door } else { Owner::Lever };
        self.use_with(owner)
    }

    async fn use_quest_tool(
        &self, _: ExactActorRef, _: UseCommand, _: UseIntent, _: Option<ItemKey>,
        _: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        Ok(Some(self.use_with(Owner::Tool)))
    }

    async fn observe_item_target(
        &self, _: ExactActorRef, _: ItemKey,
    ) -> Option<ItemTargetObservation> {
        self.use_with(Owner::Corpse);
        Some(ItemTargetObservation {
            actor: AT,
            target: UseItemTarget::Corpse { position: AT, contents: Vec::new() },
        })
    }

    async fn observe_container_with_house_capacity(
        &self, _: ExactActorRef,
        _: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        _: ItemKey, _: bool,
    ) -> Option<ContainerObservation> {
        if self.expected == Owner::Corpse {
            return None;
        }
        self.use_with(Owner::Container);
        Some(ContainerObservation::Container {
            parent: None, capacity: 8, entries: Vec::new(),
        })
    }
}

const AT: ActorPosition = ActorPosition { x: 10, y: 10, floor: 7 };

fn check(expected: Owner) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(expected))
        })
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "USE owner test panicked".to_owned())?
}

async fn scenario(expected: Owner) -> Result<(), String> {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    use crate::foundation::{ChannelId, GameSessionId, WorldId};
    let world_id = WorldId::decode(&id).map_err(|error| format!("{error:?}"))?;
    let channel_id = ChannelId::decode(&id).map_err(|error| format!("{error:?}"))?;
    let actor = ExactActorRef::transport_fixture(world_id, channel_id);
    let capabilities = [4, 6, 11, 12, 14, 15, 22];
    let offered = capabilities.map(|id| OfferedCapability { id, requires: &[] });
    let mut admitted = AdmittedSession {
        game_session_id: GameSessionId::decode(&id).map_err(|error| format!("{error:?}"))?,
        world_id,
        channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        item_fence: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(&offered, &capabilities)
                .ok_or("USE test capabilities")?,
            ..SessionContinuity::FRESH
        },
    };
    let authority = Authority { expected, seen: Cell::new(None) };
    let item = |id| ViewItem {
        key: ItemKey::Instance([id; 16]),
        appearance_id: 2854,
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    };
    let mut view = SessionItemView::resume(admitted.continuity.item_view)
        .with_container_tree().with_map_view();
    view.snapshot(InventoryItems {
        main_backpack: Some(item(1)),
        entries: vec![item(2)],
        equipment: Vec::new(),
    }, None).map_err(|error| format!("{error:?}"))?;
    let corpse = EntityRef { identity: [2; _], generation: 0 };
    view.attach_spatial_handles(&mut [WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: EntityKind::Corpse,
        entity: corpse,
        position: AT,
        detail: EntityDetail::Object {
            item_definition_ref: 1,
            quantity: 1,
            item_handle: None,
        },
    }]).map_err(|error| format!("{error:?}"))?;
    let callback = ItemKey::MapAdded(1);
    view.map_view(&[callback], |_| Ok(())).map_err(|error| format!("{error:?}"))?;
    let pack = view.table().handle(&item(2).key).ok_or("carried handle")?;
    let corpse = view.table().handle(&ItemKey::Entity(corpse.identity)).ok_or("corpse handle")?;
    let target = match expected {
        Owner::Corpse => UseTarget::Item(corpse),
        Owner::Callback => UseTarget::Item(view.table().handle(&callback).ok_or("callback handle")?),
        Owner::Door | Owner::Lever => UseTarget::WorldObject(WorldObjectTarget {
            placement: if expected == Owner::Door { b"door".to_vec() } else { b"lever".to_vec() },
            expected_revision: 0,
        }),
        _ => UseTarget::Item(pack),
    };
    let payload = encode_item_use_intent(&UseIntent {
        target,
        use_with: None,
        use_with_map_item: (expected == Owner::Tool).then_some(MapUseTarget::Item(corpse)),
        use_with_position: None,
    }).map_err(|error| format!("{error:?}"))?;
    let reply = route(&authority, actor, &mut admitted, ClientCommandView {
        command_id: 1,
        command_type: COMMAND_TYPE_USE_INTENT,
        payload: &payload,
    }, &mut Some(view), &mut Some(ContainerViewState::default()), &None).await;
    assert_eq!(authority.seen.get(), Some(expected));
    assert!(match reply {
        Some(Dispatch::Use(outcome)) => outcome.disposition == UseDisposition::Committed,
        Some(Dispatch::UseItem(decision, _)) => decision == OpenDecision::Open,
        _ => false,
    });
    Ok(())
}

macro_rules! owner_case {
    ($name:ident, $owner:ident) => {
        #[test]
        fn $name() -> Result<(), String> {
            check(Owner::$owner)
        }
    };
}

owner_case!(stone_reaches_carried_owner, Stone);
owner_case!(food_reaches_food_owner, Food);
owner_case!(corpse_reaches_corpse_view_owner, Corpse);
owner_case!(container_reaches_container_view_owner, Container);
owner_case!(cube_reaches_cube_owner, Cube);
owner_case!(rune_reaches_rune_owner, Rune);
owner_case!(map_callback_reaches_world_item_owner, Callback);
owner_case!(door_reaches_world_object_owner, Door);
owner_case!(lever_reaches_world_object_owner, Lever);
owner_case!(tool_reaches_targeted_quest_owner, Tool);
owner_case!(house_item_reaches_event_owner, HouseItem);
